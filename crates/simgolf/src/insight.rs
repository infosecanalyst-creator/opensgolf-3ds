//! OpenSGolf extras: golfer insight (extras.rs). Two observers of the simulation that only read it, each with its own tick
//! in Preferences > OpenSGolf extras:
//!
//! * the landing map: after every golfers' tick the balls in play are followed (sg_core::golfer's ball flags and places,
//!   nothing written back); when a tee shot or an approach comes to rest (a putt, struck on the green, is left out) its
//!   resting place, where it first came down, its number on the hole and the ground it stopped on go into a rolling store
//!   of the hole's last `MAX_SHOTS` shots. Shift+D (or the Information menu) shows them as dots on the course for the hole
//!   under the pointer (or the last one looked at, or the one Hole Stats last showed), coloured by the shot's number, with a
//!   legend counting where they ended up. The legend's tabs switch the dots to heat maps (the port's own, 0.5): where
//!   balls come to rest, where golfers stand waiting (`waiting`: a golfer standing where it stood last tick, its ball and
//!   its partner's still), and where golfers complain (each thought that put a golfer out, at the golfer's tile), for
//!   the hole or the whole course;
//! * comments by hole: every thought a golfer has while playing a hole (the thought bubbles, golfer.rs `event`) is
//!   tallied by hole and kind for this month, last month and this year, and a page (Shift+I, the Information menu, or a
//!   tab after F10 on the tabbed reports) lists each hole's three most frequent with their counts, in the words of the
//!   Customise screen's dialogue labels or the F2 report's lines and in the F2 report's colours of their tone.
//!
//! Both are kept with the game only while their tick is on (a save made with them off is unchanged), and cost a pass over
//! the golfer slots per tick (a few compares each) and a counter per thought.
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, draw_tab};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgba, Fnt, Screen as Ui, F_ARIAL10, F_INFO14, F_MANUAL15, F_MANUAL20};
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

/// A hole's thoughts in a period: by thought id, how often and the latest argument (for the line's words).
pub type HoleTally = BTreeMap<u8, (u32, i32)>;

/// The periods the comments page shows: this month, last month, this year.
pub const PERIODS: usize = THIS_YEAR + 1;
pub const THIS_MONTH: usize = 0;
pub const LAST_MONTH: usize = 1;
pub const THIS_YEAR: usize = 2;

/// The comments by hole: this month's tally (`month` the game's month count, tick >> 10), last month's and this year's
/// (eight months a year, tick >> 13).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tally {
    #[serde(default)]
    pub month: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub now: BTreeMap<u8, HoleTally>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub last: BTreeMap<u8, HoleTally>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub year: BTreeMap<u8, HoleTally>,
}

impl Tally {
    pub fn is_empty(&self) -> bool {
        self.now.is_empty() && self.last.is_empty() && self.year.is_empty()
    }

    /// Moves on to `month`: this month becomes last month when it is the next one, and the year's tally goes on within the
    /// same year; after a longer gap (or going back, a game loaded from earlier) nothing carries over.
    pub fn roll(&mut self, month: u32) {
        if month == self.month {
            return;
        }
        let next = month == self.month.wrapping_add(1);
        self.last = if next { std::mem::take(&mut self.now) } else { BTreeMap::new() };
        self.now.clear();
        if month < self.month || month >> 3 != self.month >> 3 {
            self.year.clear();
        }
        self.month = month;
    }

    pub fn add(&mut self, month: u32, hole: u8, id: u8, arg: i32) {
        self.roll(month);
        for m in [&mut self.now, &mut self.year] {
            let e = m.entry(hole).or_default().entry(id).or_insert((0, arg));
            e.0 += 1;
            e.1 = arg;
        }
    }

    fn period(&self, period: usize) -> &BTreeMap<u8, HoleTally> {
        match period {
            THIS_MONTH => &self.now,
            LAST_MONTH => &self.last,
            _ => &self.year,
        }
    }

    /// A hole's `n` most frequent thoughts in a period, most frequent first (ties: the lower id first), as (id, count,
    /// argument).
    pub fn top(&self, hole: u8, period: usize, n: usize) -> Vec<(u8, u32, i32)> {
        let mut v: Vec<_> = self.period(period).get(&hole).into_iter().flatten().map(|(&id, &(c, a))| (id, c, a)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    /// All the thoughts tallied on a hole in a period.
    pub fn total(&self, hole: u8, period: usize) -> u32 {
        self.period(period).get(&hole).map(|m| m.values().map(|e| e.0).sum()).unwrap_or(0)
    }
}

/// What the insight extras keep with the game.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default, skip_serializing_if = "Landings::is_empty")]
    pub landings: Landings,
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    pub tally: Tally,
    /// The landing map's heat maps of waiting and complaints (a save from before them reads as none, which fill from
    /// then on).
    #[serde(default, skip_serializing_if = "Heat::is_empty")]
    pub heat: Heat,
}

impl Kept {
    /// What a save keeps: only the parts whose tick is on, and nothing at all when neither has anything.
    pub fn for_save(k: &Option<Kept>, landing: bool, comments: bool) -> Option<Kept> {
        let k = k.as_ref()?;
        let out = Kept {
            landings: if landing { k.landings.clone() } else { Landings::default() },
            tally: if comments { k.tally.clone() } else { Tally::default() },
            heat: if landing { k.heat.clone() } else { Heat::default() },
        };
        (!out.landings.is_empty() || !out.tally.is_empty() || !out.heat.is_empty()).then_some(out)
    }
}

// ---- the heat maps --------------------------------------------------------------------------------------------------------

/// What the landing map shows: the shots as dots (the map as it was), or a heat map of where balls come to rest, where
/// golfers stand waiting, or where they complain.
pub const SHOW_DOTS: u8 = 0;
pub const SHOW_LANDED: u8 = 1;
pub const SHOW_WAITING: u8 = 2;
pub const SHOW_COMPLAINTS: u8 = 3;
pub const SHOWS: usize = 4;
const SHOW_KEYS: [&str; SHOWS] =
    ["STR_LANDING_SHOW_DOTS", "STR_LANDING_SHOW_LANDED", "STR_LANDING_SHOW_WAITING", "STR_LANDING_SHOW_COMPLAINTS"];
/// A tile's count is halved, with the rest of its hole's, when it passes this: the map leans to the recent months and its
/// counts stay small.
const HEAT_AGE: u32 = 50_000;

/// A tile as the heat maps key it (a << 16 | b, so the save's JSON keys stay short).
pub fn tile_key(a: i32, b: i32) -> u32 {
    (a.clamp(0, 0xffff) as u32) << 16 | b.clamp(0, 0xffff) as u32
}

pub fn key_tile(k: u32) -> (i32, i32) {
    ((k >> 16) as i32, (k & 0xffff) as i32)
}

/// One heat map: by hole, by tile, a count.
pub type HeatMap = BTreeMap<u8, BTreeMap<u32, u32>>;

/// The heat maps kept with the game: golfer ticks spent standing waiting (on the tee for the group ahead, for the way to
/// clear, held up on the path) and complaints (thoughts that put a golfer out), each by hole and tile, and the tee and cup
/// each hole had (a hole built again elsewhere starts afresh, as the landing map's shots do).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Heat {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub wait: HeatMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub moan: HeatMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layout: BTreeMap<u8, [i32; 4]>,
}

impl Heat {
    pub fn is_empty(&self) -> bool {
        self.wait.is_empty() && self.moan.is_empty()
    }

    /// Adds `n` to a tile of a hole laid out as `layout` in one of the maps (`complaint` the complaints' map).
    pub fn add(&mut self, complaint: bool, hole: u8, layout: [i32; 4], tile: (i32, i32), n: u32) {
        if self.layout.get(&hole).is_some_and(|l| *l != layout) {
            self.wait.remove(&hole);
            self.moan.remove(&hole);
        }
        self.layout.insert(hole, layout);
        let m = if complaint { &mut self.moan } else { &mut self.wait };
        let tiles = m.entry(hole).or_default();
        let c = tiles.entry(tile_key(tile.0, tile.1)).or_insert(0);
        *c = c.saturating_add(n);
        if *c > HEAT_AGE {
            tiles.retain(|_, v| {
                *v /= 2;
                *v > 0
            });
        }
    }
}

/// Whether a golfer standing where it stood last tick is waiting: standing or sitting (not addressing, swinging or
/// reacting), its ball still, and its partner neither playing a shot nor watching a ball in flight. Holds on the tee for the
/// group ahead, waits for the way to clear and golfers held up on the path all show so.
pub fn waiting(me: &sg_core::golfer::Golfer, partner: Option<&sg_core::golfer::Golfer>) -> bool {
    use sg_core::golfer::anim;
    if !(1..19).contains(&me.hole) || !matches!(me.anim, anim::STAND | anim::SITTING) || me.flags & flag::BALL_MOVING != 0 {
        return false;
    }
    partner.is_none_or(|p| p.flags & flag::BALL_MOVING == 0 && !(p.anim == anim::ADDRESS || (1..=6).contains(&p.anim)))
}

/// A heat map's tiles for a hole (0: every hole), summed.
fn heat_tiles(m: &HeatMap, hole: u8) -> BTreeMap<u32, u32> {
    let mut out = BTreeMap::new();
    for (_, tiles) in m.iter().filter(|(h, _)| hole == 0 || **h == hole) {
        for (&k, &v) in tiles {
            *out.entry(k).or_insert(0) += v;
        }
    }
    out
}

/// The heat maps' colour for a tile with `t` (0 to 1) of the busiest tile's count: a pale yellow through orange to a deep
/// red, more opaque as it rises, so the course shows through the quiet tiles (on grass, sand and snow alike, and on the
/// free set's art).
pub fn heat_colour(t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0).sqrt();
    let stops = [(0.0, [1.0, 0.95, 0.45, 0.30]), (0.5, [1.0, 0.55, 0.10, 0.45]), (1.0, [0.80, 0.0, 0.20, 0.62])];
    let (a, b) = if t < 0.5 { (stops[0], stops[1]) } else { (stops[1], stops[2]) };
    let f = (t - a.0) / (b.0 - a.0);
    let mut c = [0.0; 4];
    for i in 0..4 {
        c[i] = a.1[i] + (b.1[i] - a.1[i]) * f;
    }
    c
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
    /// The comments page's period (THIS_MONTH, LAST_MONTH, THIS_YEAR) and its first hole shown (an index into the open
    /// holes).
    pub period: usize,
    pub off: usize,
    /// The map keeps its hole whatever the pointer is over (the --screen landing test hook's SG_LANDING_HOLE).
    pub fixed: bool,
    /// What the map shows (SHOW_DOTS, SHOW_LANDED, SHOW_WAITING, SHOW_COMPLAINTS) and whether for the whole course.
    pub show: u8,
    pub whole: bool,
    /// Each golfer slot's person record and place after the last tick, to see who stood still (the heat map of waiting).
    pub prev: Vec<(i32, i32, i32)>,
}

/// The landing map legend's height at normal size.
pub const LEGEND_H: f32 = 186.0;

/// The landing map's legend as laid out for the text size: the box, its factor, the four What tabs, the two Where tabs
/// and the top of its key.
pub struct Legend {
    pub r: Rect,
    pub k: f32,
    pub shows: [Rect; SHOWS],
    pub scopes: [Rect; 2],
    pub body: f32,
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

/// The shot numbers' colours (the path view's 15-bit colours): white the tee shot, yellow the second, orange the third,
/// red the fourth and later.
pub const SHOT_COLOURS: [u32; SHOT_KINDS] = [0x7fff, 0x7fe0, 0x7e00, 0x7d08];
const SHOT_KEYS: [&str; SHOT_KINDS] = ["STR_LANDING_SHOT_1", "STR_LANDING_SHOT_2", "STR_LANDING_SHOT_3", "STR_LANDING_SHOT_4"];

// ---- the comments page's layout (the message history's box) ------------------------------------------------------------------

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const TOP_Y: f32 = BOX.y + 66.0;
const BOTTOM_Y: f32 = BOX.y + BOX.h - 66.0;
/// A hole's block: the hole and its count on the left, its thoughts one a line, each count at the right edge.
const HOLE_X: f32 = BOX.x + 24.0;
const TEXT_X: f32 = BOX.x + 112.0;
const COUNT_R: f32 = BOX.x + BOX.w - 58.0;
const TEXT_W: f32 = COUNT_R - TEXT_X - 40.0;
const LINE: f32 = 16.0;
/// The page's font: the reports' (Manual SSi 14, as the F2 report and the other report tabs' pages print).
const FONT: Fnt = F_INFO14;

/// The page's font and line pitch at the text size (textsize.rs): the blocks grow and the page scrolls sooner.
fn font() -> Fnt {
    crate::textsize::big(FONT)
}
fn line() -> f32 {
    crate::textsize::px(LINE)
}
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: TOP_Y, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: BOTTOM_Y - 20.0, w: 20.0, h: 20.0 };
const PERIOD_KEYS: [&str; PERIODS] = ["STR_HOLECOMMENTS_THIS_MONTH", "STR_HOLECOMMENTS_LAST_MONTH", "STR_HOLECOMMENTS_THIS_YEAR"];

fn period_tab(k: usize) -> Rect {
    Rect::new(BOX.x + 24.0 + 214.0 * k as f32, BOX.y + 40.0, 206.0, 18.0)
}

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

/// A hole's block on the page: the hole, its top, its height, how many thoughts in all, and its thoughts' lines (the words
/// wrapped to the width, the count on the first line, the colour of the line's tone).
struct Block {
    hole: usize,
    y: f32,
    h: f32,
    total: u32,
    lines: Vec<(Vec<String>, u32, [f32; 4])>,
}

/// A block's height for `lines` lines of text (at least the hole's two).
fn block_height(lines: usize) -> f32 {
    line() * lines.max(2) as f32 + 8.0
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
                let layout = self.hole_layout(f.hole as usize);
                let k = self.xs.kept.insight.get_or_insert_with(Default::default);
                k.landings.push(f.hole as u8, layout, s);
            }
        }
        self.xs.insight.flights = flights;
        self.heat_tick(n);
    }

    /// The hole's tee and cup tiles, as the heat maps and the shots keep them.
    fn hole_layout(&self, h: usize) -> [i32; 4] {
        let hr = &self.club.holes[h];
        [hr.back.0, hr.back.1, hr.pin.0, hr.pin.1]
    }

    /// After each golfers' tick: a tick of waiting for every golfer standing where it stood last tick (`waiting`). Reads
    /// the simulation only.
    fn heat_tick(&mut self, n: usize) {
        let mut prev = std::mem::take(&mut self.xs.insight.prev);
        prev.resize(n, (-1, 0, 0));
        for (i, p) in prev.iter_mut().enumerate() {
            let gl = &self.club.g[i];
            let now = (gl.roster, gl.x, gl.y);
            let still = std::mem::replace(p, now) == now;
            if !still || gl.roster < 0 {
                continue;
            }
            let partner = usize::try_from(gl.partner).ok().filter(|&q| q < n && q != i).map(|q| &self.club.g[q]);
            if !waiting(gl, partner) {
                continue;
            }
            let (h, tile) = (gl.hole as usize, (gl.x >> 10, gl.y >> 10));
            let layout = self.hole_layout(h);
            let k = self.xs.kept.insight.get_or_insert_with(Default::default);
            k.heat.add(false, h as u8, layout, tile, 1);
        }
        self.xs.insight.prev = prev;
    }

    /// A golfer's thought (the landing map on): one that put the golfer out counts on the heat map of complaints, at the
    /// tile the golfer stands on, for the hole it is about (as comments by hole counts it). Reads the simulation only.
    pub fn heat_thought(&mut self, g: usize, id: u32, arg: i32) {
        if id >= MAX_ID || g >= sg_core::golfer::SLOTS {
            return;
        }
        let gg = &self.club.g[g];
        // the newest thought's mood change is in the history it was just put in (golfer.rs event); the few kept out of it
        // (the score at a hole's end) take their line's tone
        let bad = if gg.thoughts[0] as u32 == id {
            gg.args[0] & 0xc000 == 0xc000
        } else {
            self.club.thought(&self.course, id, arg & 0x3fff, g).tone == sg_core::thoughts::Tone::Bad
        };
        if !bad {
            return;
        }
        let hole = if matches!(id, 0x13 | 0x17) && gg.thought as u32 == id { gg.thought_stamp as i32 / 11 } else { gg.hole };
        if !(1..19).contains(&hole) {
            return;
        }
        let tile = (gg.x >> 10, gg.y >> 10);
        let layout = self.hole_layout(hole as usize);
        let k = self.xs.kept.insight.get_or_insert_with(Default::default);
        k.heat.add(true, hole as u8, layout, tile, 1);
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

    /// The holes the map shows: the one picked, or every open hole for the whole course (0 for none).
    fn shown_holes(&self) -> Vec<u8> {
        if self.xs.insight.whole {
            (1..19).filter(|&h| self.club.holes[h].par != 0).map(|h| h as u8).collect()
        } else {
            vec![self.xs.insight.hole as u8].into_iter().filter(|&h| h != 0).collect()
        }
    }

    /// The heat map shown, by tile (empty for the dots).
    fn heat_shown(&self) -> BTreeMap<u32, u32> {
        let Some(kept) = self.xs.kept.insight.as_ref() else { return BTreeMap::new() };
        let holes = self.shown_holes();
        let mut out = BTreeMap::new();
        match self.xs.insight.show {
            SHOW_LANDED => {
                for &h in &holes {
                    for sh in kept.landings.shots(h) {
                        let (x, y) = sh.rest();
                        *out.entry(tile_key(x >> 10, y >> 10)).or_insert(0) += 1;
                    }
                }
            }
            SHOW_WAITING | SHOW_COMPLAINTS => {
                let m = if self.xs.insight.show == SHOW_WAITING { &kept.heat.wait } else { &kept.heat.moan };
                for &h in &holes {
                    for (k, v) in heat_tiles(m, h) {
                        *out.entry(k).or_insert(0) += v;
                    }
                }
            }
            _ => {}
        }
        out
    }

    /// The landing map over the course (draw_hud, under the golfers' names): the hole's shots as dots, or a heat map.
    pub fn draw_landing_map(&mut self, g: &mut Gfx, s: &Ui) {
        if !self.xs.insight.map || !crate::extras::landing_map() || self.screen == Screen::YearEnd {
            return;
        }
        // the hole follows the pointer over the course, not over the legend
        let (px, py) = self.info.pointer;
        if self.screen == Screen::Play && !self.xs.insight.fixed && !self.landing_legend().r.has(px, py) {
            if let Some(h) = self.hole_near_pointer() {
                self.xs.insight.hole = h;
            }
        }
        let h = self.xs.insight.hole;
        if !(1..19).contains(&h) || self.club.holes[h].par == 0 {
            self.xs.insight.hole = 0;
        }
        if self.xs.insight.show != SHOW_DOTS {
            let tiles = self.heat_shown();
            let most = tiles.values().copied().max().unwrap_or(0).max(1) as f32;
            let corner = |x: i32, y: i32| {
                let (wx, wz) = self.units_to_world(x, y);
                self.screen_of_world(wx, wz)
            };
            for (&k, &n) in &tiles {
                let (a, b) = key_tile(k);
                if self.screen_of(a * 1024 + 512, b * 1024 + 512).is_none() {
                    continue;
                }
                let (x0, y0, x1, y1) = (a * 1024, b * 1024, a * 1024 + 1024, b * 1024 + 1024);
                if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)) {
                    s.quad(g, [p0, p1, p2, p3], heat_colour(n as f32 / most));
                }
            }
            return;
        }
        let k = (self.exe_zoom() / 4.0).clamp(0.5, 1.0);
        let shade = rgba(0.0, 0.0, 0.0, 0.45);
        if let Some(kept) = self.xs.kept.insight.as_ref() {
            for h in self.shown_holes() {
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
    }

    /// The legend's place in the upper right (under the path view's legend when both are up), its tabs, and the text
    /// size it is drawn at (textsize.rs: the box grows with its text, leftwards and down).
    pub fn landing_legend(&self) -> Legend {
        // under the path view's legend when both are up, as big as the pair still fits above the dock's panel
        let (k, _, y) = self.legend_place(LEGEND_H);
        let (w, h) = ((224.0 * k).round(), (LEGEND_H * k).round());
        let r = Rect::new(790.0 - w, y, w, h);
        let (tw, th) = (((w - 24.0) / 2.0).floor(), (14.0 * k).round());
        let tab = |col: usize, row: usize| {
            Rect::new(r.x + 10.0 + (tw + 4.0) * col as f32, r.y + (28.0 * k).round() + (th + 4.0) * row as f32, tw, th)
        };
        Legend {
            r,
            k,
            shows: [tab(0, 0), tab(1, 0), tab(0, 1), tab(1, 1)],
            scopes: [tab(0, 2), tab(1, 2)],
            body: r.y + (28.0 * k).round() + (th + 4.0) * 3.0 + 6.0,
        }
    }

    /// A click on the landing map's legend (Play): its tabs pick what the map shows and for which holes; the rest of the box
    /// takes the click too, so it does not go to the course under it. False when the legend is not up or not under it.
    pub fn landing_legend_click(&mut self, vx: f32, vy: f32) -> bool {
        if !self.xs.insight.map || !crate::extras::landing_map() || self.screen != Screen::Play {
            return false;
        }
        let l = self.landing_legend();
        if !l.r.has(vx, vy) {
            return false;
        }
        if let Some(i) = l.shows.iter().position(|t| t.has(vx, vy)) {
            self.xs.insight.show = i as u8;
        } else if let Some(i) = l.scopes.iter().position(|t| t.has(vx, vy)) {
            self.xs.insight.whole = i == 1;
        }
        true
    }

    /// The landing map's legend in the upper right (draw_hud, over the golfers' names as the path view's legend is): what it
    /// shows (Shots, Landings, Waiting, Complaints), for this hole or the whole course, and the key to the dots or the heat.
    pub fn draw_landing_legend(&self, g: &mut Gfx, s: &Ui) {
        if !self.xs.insight.map || !crate::extras::landing_map() || self.screen == Screen::YearEnd {
            return;
        }
        use crate::enhanced::Anchor;
        use crate::textsize::fit;
        s.anchor(Anchor::TopRight);
        let l = self.landing_legend();
        let (r, k) = (l.r, l.k);
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        self.art.trans_fill(g, s, x, y, w, h, true);
        let (white, yellow, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x6318));
        let small = F_ARIAL10.scaled(k);
        let line = (14.0 * k).round();
        let room = w - 20.0;
        let hole = self.xs.insight.hole;
        let title = if self.xs.insight.whole {
            tr("STR_LANDING_TITLE_COURSE").to_string()
        } else if hole == 0 {
            tr("STR_LANDING_TITLE").to_string()
        } else {
            trf("STR_LANDING_TITLE_HOLE", &[("HOLE", &hole)])
        };
        s.put_shadowed(g, fit(F_MANUAL15, k, &title, room), x + 10.0, y + 6.0, &title, yellow, false);
        let (px, py) = self.info.pointer;
        for (i, t) in l.shows.iter().enumerate() {
            let label = tr(SHOW_KEYS[i]);
            draw_tab(g, s, *t, fit(F_ARIAL10, k, label, t.w - 4.0), label, self.xs.insight.show == i as u8, t.has(px, py));
        }
        for (i, t) in l.scopes.iter().enumerate() {
            let label = tr(if i == 0 { "STR_LANDING_THIS_HOLE" } else { "STR_LANDING_WHOLE_COURSE" });
            draw_tab(g, s, *t, fit(F_ARIAL10, k, label, t.w - 4.0), label, self.xs.insight.whole == (i == 1), t.has(px, py));
        }
        let put = |g: &mut Gfx, yy: f32, text: &str, c: [f32; 4]| s.put(g, fit(F_ARIAL10, k, text, room), x + 10.0, yy, text, c);
        let by = l.body;
        let holes = self.shown_holes();
        if holes.is_empty() {
            put(g, by, tr(if self.xs.insight.whole { "STR_LANDING_NO_HOLES" } else { "STR_LANDING_POINT" }), white);
        } else if self.xs.insight.show == SHOW_DOTS {
            for (i, key) in SHOT_KEYS.iter().enumerate() {
                let (rx, ry) = (x + 11.0 + ((w - 20.0) / 2.0).floor() * (i % 2) as f32, by + line * (i / 2) as f32);
                let d = (8.0 * k).round();
                s.fill(g, rx, ry + 2.0, d, d, rgba(0.0, 0.0, 0.0, 0.6));
                s.fill(g, rx + 1.0, ry + 3.0, d - 2.0, d - 2.0, c15(SHOT_COLOURS[i]));
                s.put(g, small, rx + d + 6.0, ry, tr(key), white);
            }
            let mut counts = [0u32; GROUNDS];
            if let Some(kept) = self.xs.kept.insight.as_ref() {
                for &hh in &holes {
                    for (c, n) in counts.iter_mut().zip(kept.landings.counts(hh)) {
                        *c += n;
                    }
                }
            }
            let n: u32 = counts.iter().sum();
            let ty = by + 2.0 * line + 6.0;
            if n == 0 {
                put(g, ty, tr("STR_LANDING_NONE"), white);
            } else {
                put(g, ty, &trf("STR_LANDING_SHOTS", &[("COUNT", &n)]), white);
                let c = |i: usize| counts[i];
                let l1 = trf("STR_LANDING_GROUNDS_1", &[("FAIRWAY", &c(0)), ("ROUGH", &c(1)), ("SAND", &c(2))]);
                let l2 = trf("STR_LANDING_GROUNDS_2", &[("WATER", &c(3)), ("GREEN", &c(4)), ("OTHER", &c(5))]);
                put(g, ty + line, &l1, white);
                put(g, ty + 2.0 * line, &l2, white);
            }
        } else {
            // the heat's key: a bar from the quietest colour to the busiest, few to many
            let (bx, bw, bh) = (x + 10.0, w - 20.0, (8.0 * k).round());
            let steps = 24;
            for i in 0..steps {
                let t = (i as f32 + 0.5) / steps as f32;
                let mut c = heat_colour(t);
                c[3] = 0.4 + 0.6 * c[3];
                let x0 = bx + bw * i as f32 / steps as f32;
                s.fill(g, x0, by + 2.0, bw / steps as f32 + 0.5, bh, c);
            }
            let (few, many) = (tr("STR_LANDING_FEW"), tr("STR_LANDING_MANY"));
            s.put(g, small, bx, by + bh + 4.0, few, white);
            s.put_right(g, small, bx + bw, by + bh + 4.0, many, white);
            let tiles = self.heat_shown();
            let total: u64 = tiles.values().map(|&v| v as u64).sum();
            let ty = by + bh + 4.0 + line + 4.0;
            let text = match (self.xs.insight.show, total) {
                (_, 0) => tr("STR_LANDING_HEAT_NONE").to_string(),
                (SHOW_LANDED, n) => trf("STR_LANDING_HEAT_LANDED", &[("COUNT", &n)]),
                // golfer ticks of 87 ms, as minutes at normal speed
                (SHOW_WAITING, n) => trf("STR_LANDING_HEAT_WAITING", &[("MINUTES", &(n * sg_core::flight::TICK_MS as u64 / 60_000))]),
                (_, n) => trf("STR_LANDING_HEAT_COMPLAINTS", &[("COUNT", &n)]),
            };
            let words = crate::extras::wrap(small, &text, room);
            for (i, l) in words.iter().take(2).enumerate() {
                put(g, ty + line * i as f32, l, white);
            }
        }
        put(g, y + h - line - 4.0, tr("STR_LANDING_CLOSE"), grey);
        s.anchor(Anchor::Centre);
    }

    // ---- comments by hole -----------------------------------------------------------------------------------------------

    /// A golfer's thought while playing a hole, tallied for the hole (comments by hole on): every thought below 50 (as the
    /// F2 report lists them; 50 is a story's line), good, bad or neither.
    pub fn tally_thought(&mut self, g: usize, id: u32, arg: i32) {
        if id >= MAX_ID || g >= sg_core::golfer::SLOTS {
            return;
        }
        let gg = &self.club.g[g];
        // the score at a hole's end (0x13, 0x17) shows once the golfer has moved on: its hole is the thought's stamp's
        // (hole * 11 + strokes, golfer.rs event), as thoughts.rs reads it
        let hole = if matches!(id, 0x13 | 0x17) && gg.thought as u32 == id { gg.thought_stamp as i32 / 11 } else { gg.hole };
        if !(1..19).contains(&hole) {
            return;
        }
        let month = self.game_tick >> 10;
        let k = self.xs.kept.insight.get_or_insert_with(Default::default);
        k.tally.add(month, hole as u8, id as u8, arg & 0x3fff);
    }

    /// A thought's words and tone for the page: the Customise screen's label of its dialogue row when it has one ("Hit into
    /// the water"), else the line the F2 report shows for it; the tone is the F2 line's.
    fn thought_label(&self, id: u8, arg: i32) -> (String, sg_core::thoughts::Tone) {
        use sg_core::roster::{SAYING_CODES, SAYING_LABEL_KEYS};
        let line = self.club.thought(&self.course, id as u32, arg, 0x98);
        let text = match SAYING_CODES.iter().position(|&c| c == id as u32).filter(|&r| r > 0) {
            Some(r) => tr(SAYING_LABEL_KEYS[r]).to_string(),
            None => line.text.trim().to_string(),
        };
        (text, line.tone)
    }

    /// Shift+I, the Information menu's line or the reports' eleventh tab: the comments by hole page (closing to the course).
    pub fn open_hole_comments(&mut self) {
        self.screen = Screen::HoleComments;
        self.xs.insight.off = 0;
        self.hover = -1;
    }

    fn close_hole_comments(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// The page's holes: the open ones.
    fn comment_holes(&self) -> Vec<usize> {
        (1..19).filter(|&h| self.club.holes[h].par != 0).collect()
    }

    /// The tally as of now (a month that has turned since the last thought shows as its own, empty, month).
    fn tally_now(&self) -> Tally {
        let mut t = self.xs.kept.insight.as_ref().map(|k| k.tally.clone()).unwrap_or_default();
        t.roll(self.game_tick >> 10);
        t
    }

    /// The blocks on the page from the first hole shown.
    fn comment_blocks(&self) -> Vec<Block> {
        let (navy, good, bad) = (c15(0x0018), c15(0x01e0), c15(0x5800));
        let tally = self.tally_now();
        let period = self.xs.insight.period;
        let mut out = Vec::new();
        let mut y = TOP_Y;
        for &h in self.comment_holes().iter().skip(self.xs.insight.off) {
            let mut lines = Vec::new();
            for (id, n, arg) in tally.top(h as u8, period, TOP) {
                let (text, tone) = self.thought_label(id, arg);
                if text.is_empty() {
                    continue;
                }
                // the F2 report's colours, darkened for the box's blue as the golfer profile has them
                let c = match tone {
                    sg_core::thoughts::Tone::Good => good,
                    sg_core::thoughts::Tone::Bad => bad,
                    _ => navy,
                };
                lines.push((crate::extras::wrap(font(), &text, TEXT_W), n, c));
            }
            let h_px = block_height(lines.iter().map(|l| l.0.len()).sum());
            if y + h_px > BOTTOM_Y && !out.is_empty() {
                break;
            }
            out.push(Block { hole: h, y, h: h_px, total: tally.total(h as u8, period), lines });
            y += h_px;
        }
        out
    }

    fn hole_comments_scroll(&mut self, d: i32) {
        let n = self.comment_holes().len() as i32;
        self.xs.insight.off = (self.xs.insight.off as i32 + d).clamp(0, (n - 1).max(0)) as usize;
    }

    pub fn hole_comments_wheel(&mut self, y: f32) {
        self.hole_comments_scroll(if y > 0.0 { -1 } else { 1 });
    }

    pub fn hole_comments_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_hole_comments();
        }
        if let Some(k) = (0..PERIODS).find(|&k| period_tab(k).has(x, y)) {
            self.xs.insight.period = k;
            return;
        }
        if UP.has(x, y) {
            return self.hole_comments_scroll(-1);
        }
        if DOWN.has(x, y) {
            return self.hole_comments_scroll(1);
        }
        let blocks = self.comment_blocks();
        if let Some(b) = blocks.iter().find(|b| (b.y..b.y + b.h).contains(&y) && x < UP.x).filter(|_| crate::extras::landing_map()) {
            // a hole's block: its landing map
            let h = b.hole;
            self.close_hole_comments();
            self.xs.insight.hole = h;
            self.xs.insight.map = true;
            let hr = &self.club.holes[h];
            self.look_at(((hr.back.0 + hr.pin.0) * 512 + 512, (hr.back.1 + hr.pin.1) * 512 + 512));
        }
    }

    pub fn hole_comments_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let page = self.comment_blocks().len().max(1) as i32;
        let p = &mut self.xs.insight.period;
        match k {
            KeyCode::Left => *p = (*p + PERIODS - 1) % PERIODS,
            KeyCode::Right => *p = (*p + 1) % PERIODS,
            KeyCode::Up => self.hole_comments_scroll(-1),
            KeyCode::Down => self.hole_comments_scroll(1),
            KeyCode::PageUp => self.hole_comments_scroll(-page),
            KeyCode::PageDown | KeyCode::Space => self.hole_comments_scroll(page),
            KeyCode::Home => self.xs.insight.off = 0,
            KeyCode::End => self.hole_comments_scroll(18),
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
        for (k, key) in PERIOD_KEYS.iter().enumerate() {
            let m = month.wrapping_sub((k == LAST_MONTH) as u32);
            let name = tr(crate::hud_ui::MONTH_KEYS[(m & 7) as usize]);
            let label = trf(key, &[("MONTH", &name), ("YEAR", &(2001 + (month >> 3)))]);
            let r = period_tab(k);
            draw_tab(g, &s, r, crate::textsize::fit(FONT, 1.25, &label, r.w - 6.0), &label, self.xs.insight.period == k, r.has(px, py));
        }
        let holes = self.comment_holes();
        if holes.is_empty() {
            s.put_centered(g, font(), 400.0, 280.0, tr("STR_HOLECOMMENTS_NO_HOLES"), white);
        }
        let blocks = self.comment_blocks();
        let clickable = crate::extras::landing_map();
        for b in &blocks {
            if clickable && (b.y..b.y + b.h).contains(&py) && (BOX.x..UP.x).contains(&px) {
                s.fill(g, BOX.x + 16.0, b.y - 2.0, UP.x - BOX.x - 24.0, b.h - 2.0, rgba(1.0, 1.0, 1.0, 0.18));
            }
            s.put(g, font(), HOLE_X, b.y, &trf("STR_HOLECOMMENTS_HOLE", &[("HOLE", &b.hole)]), white);
            s.put(g, font(), HOLE_X, b.y + line(), &trf("STR_HOLECOMMENTS_HEARD", &[("COUNT", &b.total)]), navy);
            if b.lines.is_empty() {
                s.put(g, font(), TEXT_X, b.y, tr("STR_HOLECOMMENTS_NOTHING"), grey);
            }
            let mut y = b.y;
            for (words, n, c) in &b.lines {
                let count = n.to_string();
                s.put(g, font(), COUNT_R - font().width(&count), y, &count, *c);
                for w in words {
                    s.put(g, font(), TEXT_X, y, w, *c);
                    y += line();
                }
            }
            s.fill(g, BOX.x + 20.0, b.y + b.h - 5.0, UP.x - BOX.x - 32.0, 1.0, rgba(0.0, 0.0, 0.3, 0.25));
        }
        // the scroll arrows and where the page is among the holes (the message history's)
        let n = holes.len();
        let can_up = self.xs.insight.off > 0;
        let can_down = blocks.last().is_some_and(|b| holes.last() != Some(&b.hole));
        for (r, up, ok) in [(UP, true, can_up), (DOWN, false, can_down)] {
            draw_button(g, &s, r, "", ok, r.has(px, py));
            draw_arrow(g, &s, r.x + r.w / 2.0, r.y + r.h / 2.0, up, if ok { navy } else { grey });
        }
        if n > 0 {
            let track = DOWN.y - UP.y - UP.h - 8.0;
            let th = (track * blocks.len().max(1) as f32 / n as f32).max(8.0).min(track);
            let ty =
                UP.y + UP.h + 4.0 + (track - th) * (self.xs.insight.off as f32 / n.saturating_sub(blocks.len()).max(1) as f32).min(1.0);
            s.fill(g, UP.x + 7.0, ty, 6.0, th, white);
        }
        let help = if clickable { "STR_HOLECOMMENTS_HELP_MAP" } else { "STR_HOLECOMMENTS_HELP" };
        // the foot's two lines keep to the room left of the OK tick (bigger text only as far as fits; a line wider than the
        // room, a translation, a little smaller or cut: ui.rs fit_line, issue #164)
        let room = ok_tick().x - 6.0 - (BOX.x + 24.0);
        for (t, y) in [(tr("STR_HOLECOMMENTS_COUNTED"), BOX.y + BOX.h - 46.0), (tr(help), BOX.y + BOX.h - 28.0)] {
            s.put_fit(g, crate::textsize::fit(FONT, 1.25, t, room), BOX.x + 24.0, y, room, t, navy);
        }
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
    fn the_tally_keeps_this_month_last_month_and_the_year() {
        let mut t = Tally::default();
        t.add(5, 1, 0x0d, 17);
        t.add(5, 1, 0x0d, 17);
        t.add(5, 1, 0x1c, 0);
        for _ in 0..3 {
            t.add(5, 1, 0x04, 0);
        }
        t.add(5, 2, 0x1c, 0);
        let ids = |v: Vec<(u8, u32, i32)>| v.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>();
        assert_eq!(ids(t.top(1, THIS_MONTH, 3)), [(0x04, 3), (0x0d, 2), (0x1c, 1)]);
        assert_eq!(t.top(1, THIS_MONTH, 2).len(), 2);
        assert_eq!(t.total(1, THIS_MONTH), 6);
        assert_eq!(t.total(2, THIS_MONTH), 1);
        // the next month: this month becomes last month, the year goes on
        t.add(6, 1, 0x1c, 0);
        assert_eq!(t.total(1, LAST_MONTH), 6);
        assert_eq!(t.total(1, THIS_MONTH), 1);
        assert_eq!(t.total(1, THIS_YEAR), 7);
        assert_eq!(ids(t.top(1, THIS_YEAR, 1)), [(0x04, 3)]);
        assert_eq!(t.top(2, LAST_MONTH, 3), [(0x1c, 1, 0)]);
        // a new year (month 8 is March of the second year): the year starts afresh, last month carries over
        t.roll(7);
        t.add(8, 3, 0x10, 0);
        assert_eq!(t.total(1, THIS_YEAR), 0);
        assert_eq!(t.total(3, THIS_YEAR), 1);
        // a gap of more than a month: nothing carries over
        t.roll(20);
        assert!(t.is_empty());
        // ties: the lower id first
        t.add(20, 4, 9, 0);
        t.add(20, 4, 2, 0);
        assert_eq!(t.top(4, THIS_MONTH, 3)[0].0, 2);
    }

    #[test]
    fn blocks_hold_the_holes_lines() {
        assert_eq!(block_height(0), block_height(2));
        assert_eq!(block_height(3), LINE * 3.0 + 8.0);
        // eighteen holes need more than one page: the list scrolls
        assert!(block_height(3) * 18.0 > BOTTOM_Y - TOP_Y);
    }

    #[test]
    fn a_save_keeps_only_what_is_on() {
        let mut k = Kept::default();
        k.landings.push(1, [0; 4], Shot::new((0, 0), (0, 0), 1, FAIRWAY));
        k.tally.add(0, 1, 4, 0);
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
    fn heat_maps_count_age_and_start_afresh() {
        let mut h = Heat::default();
        assert!(h.is_empty());
        let lay = [1, 2, 10, 12];
        h.add(false, 3, lay, (5, 7), 4);
        h.add(false, 3, lay, (5, 7), 1);
        h.add(true, 3, lay, (6, 7), 1);
        h.add(false, 4, [0; 4], (9, 9), 2);
        assert_eq!(h.wait[&3][&tile_key(5, 7)], 5);
        assert_eq!(heat_tiles(&h.wait, 0).values().sum::<u32>(), 7);
        assert_eq!(heat_tiles(&h.wait, 4).len(), 1);
        assert_eq!(heat_tiles(&h.moan, 3)[&tile_key(6, 7)], 1);
        // a busy tile halves its hole's counts, and the quiet ones go
        h.add(false, 3, lay, (1, 1), 1);
        h.add(false, 3, lay, (5, 7), HEAT_AGE);
        assert_eq!(h.wait[&3].get(&tile_key(1, 1)), None);
        assert!(h.wait[&3][&tile_key(5, 7)] <= HEAT_AGE / 2 + 3);
        // the hole built again elsewhere starts afresh, both maps
        h.add(false, 3, [7, 7, 9, 9], (2, 2), 1);
        assert_eq!(h.wait[&3].len(), 1);
        assert!(!h.moan.contains_key(&3));
        assert_eq!(key_tile(tile_key(300, 17)), (300, 17));
        // saved and read back; a save from before the heat maps reads as none
        let k = Kept { heat: h.clone(), ..Default::default() };
        let json = serde_json::to_string(&Kept::for_save(&Some(k.clone()), true, false).unwrap()).unwrap();
        assert_eq!(serde_json::from_str::<Kept>(&json).unwrap().heat, h);
        assert_eq!(Kept::for_save(&Some(k), false, true), None);
        assert!(serde_json::from_str::<Kept>("{\"landings\": {}}").unwrap().heat.is_empty());
    }

    #[test]
    fn who_is_waiting() {
        use sg_core::golfer::{anim, Golfer};
        let me = Golfer { hole: 2, anim: anim::STAND, ..Default::default() };
        assert!(waiting(&me, None));
        assert!(waiting(&Golfer { anim: anim::SITTING, ..me.clone() }, None));
        // playing, walking home, or swinging is not waiting
        assert!(!waiting(&Golfer { hole: 19, ..me.clone() }, None));
        assert!(!waiting(&Golfer { anim: anim::ADDRESS, ..me.clone() }, None));
        assert!(!waiting(&Golfer { flags: flag::BALL_MOVING, ..me.clone() }, None));
        // nor watching the partner play
        let p = Golfer { hole: 2, anim: anim::STAND, ..Default::default() };
        assert!(waiting(&me, Some(&p)));
        assert!(!waiting(&me, Some(&Golfer { anim: 3, ..p.clone() })));
        assert!(!waiting(&me, Some(&Golfer { flags: flag::BALL_MOVING, ..p.clone() })));
    }

    #[test]
    fn heat_rises_in_colour_and_opacity() {
        let (lo, hi) = (heat_colour(0.0), heat_colour(1.0));
        assert!(lo[3] < hi[3] && hi[3] < 0.7 && lo[3] > 0.2);
        assert!(lo[1] > hi[1]);
        let mut last = 0.0;
        for i in 0..=10 {
            let a = heat_colour(i as f32 / 10.0)[3];
            assert!(a >= last);
            last = a;
        }
    }

    #[test]
    fn segment_distance() {
        assert!((seg_dist(5.0, 3.0, (0.0, 0.0), (10.0, 0.0)) - 3.0).abs() < 1e-5);
        assert!((seg_dist(-4.0, 3.0, (0.0, 0.0), (10.0, 0.0)) - 5.0).abs() < 1e-5);
        assert!(seg_dist(2.0, 2.0, (2.0, 2.0), (2.0, 2.0)).abs() < 1e-5);
    }
}
