//! Course staff, as the publisher's golf.exe runs them (docs/PUBLISHER_EXE_NOTES.md, "Staff").
//!
//! Each employee has a post (the tile the player puts them on) and walks the course in the exe's own units: 1024 per tile,
//! one update per game tick. Every update an idle employee looks for work near the post: the Club Pro chats to golfers, the
//! Ranger hurries slow golfers, the Groundskeeper pulls weeds and the Soda Vendor sells drinks to thirsty golfers. The player's
//! own pro ("Gary Golf" by default) does all four jobs and walks to the last tile the player clicked. Employees walk tile by tile
//! with the exe's weighted path search, at a speed set by the ground under them. The code is our own.

use crate::land::{ExeRng, TYPES};
use crate::terrain::Terrain;

pub const UNIT: i32 = 1024;
const DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// Job codes (the exe stores them as a signed byte). The hire menu maps employee kind k (0 Club Pro, 1 Ranger,
/// 2 Groundskeeper, 3 Soda Vendor) to job -2 - k.
pub mod job {
    pub const CLUB_PRO: i8 = -2;
    pub const RANGER: i8 = -3;
    pub const GROUNDSKEEPER: i8 = -4;
    pub const SODA_VENDOR: i8 = -5;
    /// The player's own golf pro.
    pub const OWNER: i8 = -6;
}

/// Animation states: 7..10 walking, 11 standing, 12 working (the action clip).
pub const ANIM_STAND: i16 = 11;
pub const ANIM_ACTION: i16 = 12;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Employee {
    pub active: bool,
    pub job: i8,
    /// Hired as the experienced version (Celebrity, Marshall, Technician, Refresher).
    pub upgraded: bool,
    /// Walking back to the clubhouse to leave.
    pub going_home: bool,
    pub x: i32,
    pub y: i32,
    /// Post tile, or None.
    pub post: Option<(i32, i32)>,
    /// Facing 0..7 (0 is -b, clockwise).
    pub dir: i8,
    /// Movement steps left before the next path decision.
    pub steps: i16,
    /// Ticks to wait before acting again.
    pub wait: i16,
    pub anim: i16,
    pub frame: i16,
    /// The pending action (job code of what was done) and a countdown (every 4 ticks) to its effect.
    pub action: i8,
    pub countdown: u8,
    /// The golfer last served.
    pub target: i16,
    /// How many golfers this employee has served.
    pub served: i16,
    /// The month hired (game tick >> 10, +0x12) and the wages paid so far in units of $100 (+0x14): the routing map's
    /// employee list shows both.
    #[serde(default)]
    pub hired: i32,
    #[serde(default)]
    pub paid: i32,
}

/// What the staff code needs to know about, and may change on, a golfer.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StaffGolfer {
    /// On the course and playing.
    pub present: bool,
    pub x: i32,
    pub y: i32,
    pub hunger: i32,
    pub thirst: i32,
    pub fatigue: i32,
    /// Hurried along by a Ranger: walks one speed step faster.
    pub hurried: bool,
    /// Past the second shot of the hole (the Ranger only hurries golfers with at most one stroke taken).
    pub busy: bool,
    /// The last thing a Club Pro said to the golfer (mood event 0x22 or 0x3a), 0 if none.
    pub last_pro_event: u32,
    /// Negative: the golfer stands still and talks; counts up by one every other tick.
    pub pause: i32,
    /// Facing set when an employee talks to the golfer (0..7), or -1.
    pub face: i8,
}

/// Things the staff update does that the game must carry out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StaffEvent {
    /// A golfer mood event (golfer, event, argument, whether the counter test passes).
    Mood { golfer: usize, event: u32, arg: i32, counter_ok: bool },
    /// A drink sold: 2 money units under Food/Drink.
    DrinkSold { golfer: usize },
    /// A sound slot played at a map position.
    Sound { slot: i32, x: i32, y: i32 },
    /// A weed was pulled from a tile.
    WeedPulled { a: i32, b: i32 },
    /// The employee walked off the course.
    Left { employee: usize },
}

/// Per-tile state the staff code reads and changes: weeds (0x800) and "being worked on" (0x4000), plus their counters.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TileState {
    pub w: i32,
    pub h: i32,
    pub flags: Vec<u16>,
    pub counter: Vec<u8>,
}

pub const WEEDS: u16 = 0x800;
pub const WORKED: u16 = 0x4000;
/// Creek and garden tiles (the generator's 0x1000 flag), where weeds can start without a neighbouring weed.
pub const SEEDS_WEEDS: u16 = 0x1000;

impl TileState {
    pub fn new(w: i32, h: i32) -> TileState {
        TileState { w, h, flags: vec![0; (w * h) as usize], counter: vec![0; (w * h) as usize] }
    }
    fn i(&self, a: i32, b: i32) -> usize {
        (b * self.w + a) as usize
    }
}

// ---- the exe's integer geometry ---------------------------------------------------------------------------------------------

/// Cheap distance: the longer leg plus half the shorter.
pub fn approx_dist(x: i32, y: i32) -> i32 {
    let (x, y) = (x.abs(), y.abs());
    if y < x {
        (y + x * 2) / 2
    } else {
        (x + y * 2) / 2
    }
}

/// Euclidean distance with the exe's guard against overflow (legs above 0x4000 are divided by 8 first).
pub fn true_dist(x: i32, y: i32) -> i32 {
    let (mut x, mut y, mut s) = (x, y, 1);
    if x.abs() > 0x4000 {
        x /= 8;
        s = 8;
    }
    if y.abs() > 0x4000 {
        y /= 8;
        s *= 8;
    }
    (((x as i64 * x as i64 + y as i64 * y as i64) as f64).sqrt() * s as f64) as i32
}

/// Direction 0..7 of a vector, by octant (the exe's quick version).
pub fn octant(x: i32, y: i32) -> i32 {
    let (ax, ay) = (x.abs(), y.abs());
    let (mut d, near_x, near_y);
    if x < 1 {
        if y < 1 {
            d = 7;
            near_x = 6;
            near_y = 0;
        } else {
            d = 5;
            near_x = 6;
            near_y = 4;
        }
    } else if y < 1 {
        d = 1;
        near_x = 2;
        near_y = 0;
    } else {
        d = 3;
        near_x = 2;
        near_y = 4;
    }
    if ay * 2 < ax {
        d = near_x;
    }
    if ax * 2 < ay {
        return near_y;
    }
    d
}

/// Binary angle of a vector (a full turn is 2^32), the exe's polynomial arctangent.
pub fn angle(x: i32, y: i32) -> u32 {
    let ny = y.wrapping_neg();
    if x == 0 {
        return if ny < 1 { 0x8000_0000 } else { 0 };
    }
    if ny == 0 {
        return if x < 1 { 0xc000_0000 } else { 0x4000_0000 };
    }
    let ax = x.abs();
    let ay = ny.abs();
    let swap = ay < ax;
    let (hi, lo) = if swap { (ax, ay) } else { (ay, ax) };
    let r = (lo * 0x4000) / hi;
    let k = 0x1333 - r;
    let a = ((0x2800 - ((k.abs() * 0xb00) >> 0xe)) * r) >> 0xe;
    let v: i32 = if x < 1 {
        if ny < 1 {
            if !swap {
                (a + 0x8000) << 16
            } else {
                (0xc000 - a) << 16
            }
        } else if swap {
            (a + 0xc000) << 16
        } else {
            (-a) << 16
        }
    } else if ny < 1 {
        if !swap {
            (0x8000 - a) << 16
        } else {
            (a + 0x4000) << 16
        }
    } else if swap {
        (0x4000 - a) << 16
    } else {
        a << 16
    };
    v as u32
}

/// Facing 0..7 for an angle.
pub fn heading(angle: u32) -> i8 {
    ((((angle >> 28) & 0xf) + 1) >> 1 & 7) as i8
}

// ---- path search ------------------------------------------------------------------------------------------------------------

fn tile_ok(t: &Terrain, a: i32, b: i32) -> bool {
    t.inside(a, b) && t.type_at(a, b) != 20
}

fn layer(t: &Terrain, a: i32, b: i32) -> i32 {
    let ty = t.type_at(a, b);
    TYPES.get(ty as usize).map(|i| i.layer as i32).unwrap_or(4)
}

fn class(ty: i32) -> u8 {
    TYPES.get(ty as usize).map(|i| i.class).unwrap_or(0)
}

fn on_path(t: &Terrain, a: i32, b: i32) -> bool {
    t.inside(a, b) && t.path_at(a, b) != 0
}

fn footprint(t: &Terrain, a: i32, b: i32) -> bool {
    let ty = t.type_at(a, b);
    ty == 21 || ty == 22
}

/// Which way to step from `from` towards `to` (both in units): a flood from the target tile weighted by the ground (the per
/// type walking cost, diagonals one more, open water and out of bounds 16 more near the target), with paths preferred when
/// the target is a few tiles away. Staff also avoid greens, fairways, tees and sand (one more) and halve the cost elsewhere.
/// `avoid` is a tile that costs 2 more (the exe passes another walker's tile). Returns a direction, or -1 when no neighbour
/// is closer.
pub fn path_step(t: &Terrain, to: (i32, i32), from: (i32, i32), avoid: Option<(i32, i32)>, tick: u32, rng: &mut ExeRng) -> i32 {
    let (w, h) = (t.w, t.h);
    let direct = octant(to.0 - from.0, to.1 - from.1);
    let cl = |v: i32, hi: i32| v.clamp(0, hi - 1);
    let (ta, tb) = (cl(to.0 >> 10, w), cl(to.1 >> 10, h));
    let (fa, fb) = (cl(from.0 >> 10, w), cl(from.1 >> 10, h));
    let avoid = avoid.map(|(x, y)| (cl(x >> 10, w), cl(y >> 10, h))).filter(|&p| p != (ta, tb));
    if (ta, tb) == (fa, fb) {
        return direct;
    }
    let (va, vb) = (ta - fa, tb - fb);
    let span = true_dist(va, vb);
    let b5 = ((((angle(va, vb) as i32) >> 28) + 1) ^ -7) >> 1;
    let b5 = b5 as u8;
    let prefer: u32 = (1 << (((b5 & 7) + 1) & 6)) | (1 << (b5 & 6));
    let n = (w * h) as usize;
    let mut cost = vec![0u8; n];
    let idx = |a: i32, b: i32| (b * w + a) as usize;
    let mut qa = [0i32; 1024];
    let mut qb = [0i32; 1024];
    let (mut head, mut tail) = (0usize, 1usize);
    qa[0] = ta;
    qb[0] = tb;
    cost[idx(ta, tb)] = 1;
    let mut limit = 160 + 90;
    let path_rule = ((!tick & 0x40) | 0x80) >> 6;
    loop {
        let (a, b) = (qa[head], qb[head]);
        head = (head + 1) & 0x3ff;
        let c = cost[idx(a, b)] as i32;
        if c <= limit {
            for k in 0..8 {
                let (na, nb) = (a + DX[k], b + DY[k]);
                if na < 0 || nb < 0 || na >= w || nb >= h {
                    continue;
                }
                let ni = idx(na, nb);
                let old = cost[ni] as i32;
                let mut step = layer(t, na, nb) + (k as i32 & 1);
                if avoid == Some((na, nb)) {
                    step += 2;
                }
                if k & 1 == 0 && (path_rule as i32) < span && on_path(t, na, nb) && !footprint(t, na, nb) && on_path(t, a, b) {
                    let away = along_axis(na - ta, nb - tb);
                    step = (k as i32 != away) as i32;
                }
                let ty = t.type_at(na, nb);
                if ty == 1 || ty == 2 || ty == 0 || class(ty) == 7 {
                    step += 1;
                } else {
                    step = (step + 1) >> 1;
                }
                if (ty == 17 || ty == 20) && !on_path(t, na, nb) && c < 0x40 {
                    step += 16;
                }
                let total = step + c;
                if (old == 0 || total < old) && total < 0x100 {
                    cost[ni] = total as u8;
                    qa[tail] = na;
                    qb[tail] = nb;
                    tail = (tail + 1) & 0x3ff;
                    if (na, nb) == (fa, fb) {
                        limit = total;
                    }
                }
            }
        }
        if head == tail {
            break;
        }
    }
    let here = idx(fa, fb);
    let mut best = cost[here] as i32 * 2 + 1;
    let mut out = -1;
    let r = rng.below(8);
    let water_bridge = t.type_at(fa, fb) == 17 && on_path(t, fa, fb);
    for i in 0..8 {
        let k = ((i + r) & 7) as usize;
        let (na, nb) = (fa + DX[k], fb + DY[k]);
        if !tile_ok(t, na, nb) || cost[idx(na, nb)] == 0 {
            continue;
        }
        let mut v = (k as i32 & 1) + cost[idx(na, nb)] as i32 * 2;
        if water_bridge && k & 1 == 1 {
            continue;
        }
        if prefer & (1 << (k ^ 4)) != 0 {
            v -= 1;
        }
        if v < best {
            out = k as i32;
            best = v;
        }
    }
    out
}

/// Direction 0, 2, 4 or 6 along the longer axis of a vector.
fn along_axis(x: i32, y: i32) -> i32 {
    if y.abs() < x.abs() {
        if x < 1 {
            6
        } else {
            2
        }
    } else if y < 1 {
        0
    } else {
        4
    }
}

// ---- hiring -----------------------------------------------------------------------------------------------------------------

/// A new employee at the clubhouse (its anchor tile `club`), facing roughly the camera's way.
pub fn create(list: &mut Vec<Employee>, job: i8, club: (i32, i32), rng: &mut ExeRng) -> usize {
    let i = list.iter().position(|e| !e.active).unwrap_or(list.len());
    if i == list.len() {
        list.push(Employee::default());
    }
    let turn = rng.below(5);
    let e = &mut list[i];
    *e = Employee::default();
    e.x = club.0 * UNIT + 0x600;
    e.y = club.1 * UNIT + 0x600;
    e.active = true;
    e.dir = ((turn - 2) & 7) as i8;
    e.job = job;
    e.anim = ANIM_STAND;
    e.wait = rng.below(32) as i16;
    e.post = None;
    e.target = -1;
    i
}

/// Hires an employee of kind 0..3 (Club Pro, Ranger, Groundskeeper, Soda Vendor); the post is 3 tiles off the clubhouse
/// anchor on each axis, either side at random.
pub fn hire(list: &mut Vec<Employee>, kind: usize, upgraded: bool, club: (i32, i32), rng: &mut ExeRng) -> usize {
    let i = create(list, -2 - kind as i8, club, rng);
    let pa = if rng.below(2) != 0 { 3 } else { -3 };
    let pb = if rng.below(2) != 0 { 3 } else { -3 };
    list[i].post = Some((club.0 + pa, club.1 + pb));
    list[i].upgraded = upgraded;
    i
}

/// Sound slot of an employee's action: 0x5c Club Pro, 0x5d Ranger, 0x5e Groundskeeper, 0x5f Soda Vendor; +4 when upgraded,
/// +8 for the player's pro.
fn action_sound(base: i32, extra: i32) -> i32 {
    base + extra
}

// ---- the update -------------------------------------------------------------------------------------------------------------

pub struct World<'a> {
    pub terrain: &'a Terrain,
    pub tiles: &'a mut TileState,
    pub golfers: &'a mut [StaffGolfer],
    /// Clubhouse anchor tile.
    pub club: (i32, i32),
    /// Last tile the player clicked (the player's pro walks there), or None.
    pub clicked: Option<(i32, i32)>,
    pub tick: u32,
    pub difficulty: i32,
    /// Frames in the theme's weed animation (a weed being worked on starts its counter at this minus 3).
    pub weed_frames: i32,
}

/// One game tick for every employee.
pub fn tick(list: &mut [Employee], w: &mut World, rng: &mut ExeRng, out: &mut Vec<StaffEvent>) {
    for i in 0..list.len() {
        if !list[i].active {
            continue;
        }
        effects(list, i, w, out);
        if list[i].wait != 0 {
            list[i].wait -= 1;
            continue;
        }
        let e = list[i];
        let (target, delta) = if e.steps != 0 || e.going_home {
            ((e.x, e.y), (0, 0))
        } else {
            match decide(list, i, w, rng, out) {
                Some(t) => t,
                None => continue,
            }
        };
        walk(list, i, target, delta, w, rng, out);
    }
}

/// The countdown after an action: four counts in, the Club Pro's chat and the Soda Vendor's drink take effect.
fn effects(list: &mut [Employee], i: usize, w: &mut World, out: &mut Vec<StaffEvent>) {
    let e = &mut list[i];
    if e.countdown == 0 || w.tick & 3 != 0 {
        return;
    }
    e.countdown -= 1;
    let gi = e.target;
    if e.countdown != 4 || gi < 0 || gi as usize >= w.golfers.len() {
        return;
    }
    let gi = gi as usize;
    let g = &mut w.golfers[gi];
    if e.action == job::CLUB_PRO {
        let content = g.thirst < 0x11 && g.hunger < 0x11 && g.fatigue < 0xa1;
        let event = if e.upgraded || content { 0x22 } else { 0x3a };
        g.last_pro_event = event;
        out.push(StaffEvent::Mood { golfer: gi, event, arg: 0, counter_ok: false });
    }
    if e.action == job::SODA_VENDOR {
        if e.upgraded {
            g.thirst = 99;
        }
        out.push(StaffEvent::Mood { golfer: gi, event: crate::mood::ev::DRINK, arg: 0x14, counter_ok: g.thirst > 7 });
        g.thirst = 0;
        out.push(StaffEvent::DrinkSold { golfer: gi });
    }
}

/// An idle employee picks what to do. Returns a place to walk to (and the vector towards it), or None when it acted or waits.
fn decide(list: &mut [Employee], i: usize, w: &mut World, rng: &mut ExeRng, out: &mut Vec<StaffEvent>) -> Option<((i32, i32), (i32, i32))> {
    let e = list[i];
    let jb = e.job;
    let (mut tx, mut ty) = (e.x, e.y);
    let mut best = 0xfffff;
    let mut found: i32 = -1;
    let mut kind: i8 = -1;
    let mut weed: Option<(i32, i32)> = None;
    let post = e.post.unwrap_or((-1, -1));
    if e.post.is_some() {
        tx = post.0 * UNIT + 0x200;
        ty = post.1 * UNIT + 0x200;
    }
    if jb == job::GROUNDSKEEPER || jb == job::OWNER {
        let mut reach = if e.upgraded { 0x20 } else { 0x18 };
        for da in -16..=16 {
            for db in -16..=16 {
                let (a, b) = (post.0 + da, post.1 + db);
                if !tile_ok(w.terrain, a, b) || w.tiles.flags[w.tiles.i(a, b)] & WEEDS == 0 {
                    continue;
                }
                let d = approx_dist(a * 2 - post.0 - (e.x >> 10), b * 2 - post.1 - (e.y >> 10));
                if d < reach {
                    tx = a * UNIT + 0x200;
                    ty = b * UNIT + 0x200;
                    found = 1;
                    kind = job::GROUNDSKEEPER;
                    reach = d;
                    weed = Some((a, b));
                }
            }
        }
    }
    if (jb != job::GROUNDSKEEPER && jb != job::OWNER) || (weed.is_none() && jb == job::OWNER) {
        tx = post.0 * UNIT + 0x200;
        ty = post.1 * UNIT + 0x200;
        let mut pick: i32 = -1;
        for (gi, g) in w.golfers.iter().enumerate() {
            let d = true_dist(g.x - post.0 * UNIT - 0x200, g.y - post.1 * UNIT - 0x200) * 25 / 1024;
            let under = if tile_ok(w.terrain, g.x >> 10, g.y >> 10) { w.terrain.type_at(g.x >> 10, g.y >> 10) } else { 20 };
            if !g.present || gi as i16 == e.target || under == 0 || under == 22 {
                continue;
            }
            let mut cand: i8 = -1;
            if (jb == job::RANGER || jb == job::OWNER) && !(g.hurried || g.busy) {
                cand = job::RANGER;
            }
            if (jb == job::CLUB_PRO || jb == job::OWNER) && g.last_pro_event != 0x22 && g.last_pro_event != 0x3a {
                cand = job::CLUB_PRO;
            }
            if (jb == job::SODA_VENDOR || jb == job::OWNER) && g.thirst > if e.upgraded { 0 } else { 4 } {
                cand = job::SODA_VENDOR;
            }
            if (cand == jb || jb == job::OWNER) && d < best {
                pick = gi as i32;
                kind = cand;
                best = d;
            }
        }
        if best < 100 {
            let g = &w.golfers[pick as usize];
            tx = g.x;
            ty = g.y;
            found = pick;
        } else {
            if pick != -1 {
                let g = &w.golfers[pick as usize];
                list[i].dir = octant(g.x - e.x, g.y - e.y) as i8;
            }
            found = -1;
        }
    }
    if jb == job::OWNER {
        match w.clicked {
            Some((a, b)) if w.tick & 0x200 == 0 && w.terrain.type_at(a, b) != 17 => {
                tx = a * UNIT + 0x200;
                ty = b * UNIT + 0x200;
            }
            _ => {
                tx = e.x;
                ty = e.y;
            }
        }
    }
    let (dx, dy) = (tx - e.x, ty - e.y);
    if approx_dist(dx, dy) > 0x400 {
        return Some(((tx, ty), (dx, dy)));
    }
    // Arrived.
    let e = &mut list[i];
    e.anim = ANIM_STAND;
    e.wait = rng.below(16) as i16 + 1;
    e.steps = 0;
    let mut extra = if e.upgraded { 4 } else { 0 };
    if e.job == job::OWNER && kind != job::CLUB_PRO {
        extra = 8;
    }
    if found == -1 {
        e.wait = rng.below(8) as i16;
        return None;
    }
    e.dir = heading(angle(dx, dy));
    e.wait += 0x40;
    e.countdown = 7;
    e.target = found as i16;
    let mut served = true;
    match kind {
        job::SODA_VENDOR => {
            e.action = job::SODA_VENDOR;
            e.anim = ANIM_ACTION;
            e.frame = 0;
            out.push(StaffEvent::Sound { slot: action_sound(0x5f, extra), x: e.x, y: e.y });
            e.wait -= 0x20;
            let g = &mut w.golfers[found as usize];
            g.pause = -32;
            g.face = e.dir ^ 4;
        }
        job::GROUNDSKEEPER => {
            served = false;
            e.action = job::GROUNDSKEEPER;
            e.anim = ANIM_ACTION;
            e.wait = (!e.upgraded) as i16;
            if let Some((a, b)) = weed {
                let ti = w.tiles.i(a, b);
                if w.tiles.flags[ti] & WORKED == 0 {
                    w.tiles.flags[ti] |= WORKED;
                    w.tiles.counter[ti] = (w.weed_frames - 3) as u8;
                    out.push(StaffEvent::Sound { slot: action_sound(0x5e, extra), x: e.x, y: e.y });
                } else {
                    let c = w.tiles.counter[ti].wrapping_sub(1);
                    w.tiles.counter[ti] = c;
                    if c < 2 {
                        e.wait = 8;
                        w.tiles.flags[ti] &= 0xb7ff;
                        served = true;
                        out.push(StaffEvent::WeedPulled { a, b });
                    }
                }
            }
        }
        job::RANGER => {
            e.action = job::RANGER;
            let g = &mut w.golfers[found as usize];
            g.hurried = true;
            e.anim = ANIM_ACTION;
            e.frame = 0;
            out.push(StaffEvent::Sound { slot: action_sound(0x5d, extra), x: e.x, y: e.y });
        }
        job::CLUB_PRO => {
            e.action = job::CLUB_PRO;
            e.wait += 0x40;
            out.push(StaffEvent::Sound { slot: action_sound(0x5c, extra), x: e.x, y: e.y });
            e.anim = ANIM_ACTION;
            e.frame = 0;
            let g = &mut w.golfers[found as usize];
            g.pause -= 16;
            g.face = e.dir ^ 4;
        }
        _ => served = false,
    }
    if served {
        e.served += 1;
    }
    if e.wait > 1 && e.upgraded {
        e.wait /= 2;
    }
    None
}

/// Walks one tick towards `target`: a new path decision when the last one's steps are used up, then a step whose length
/// depends on the ground (6 minus its walking cost, 2..5; 6 on a path; +2 when upgraded and for the player's pro; 3 when
/// leaving), giving way to other employees just ahead.
fn walk(
    list: &mut [Employee],
    i: usize,
    target: (i32, i32),
    delta: (i32, i32),
    w: &mut World,
    rng: &mut ExeRng,
    out: &mut Vec<StaffEvent>,
) {
    let mut target = target;
    if list[i].going_home {
        target = (w.club.0 * UNIT + 0x200, w.club.1 * UNIT + 0x200);
        if (i as u32 * 0x23 + w.tick).is_multiple_of(100) {
            list[i].countdown = 7;
            list[i].action = 0x23;
        }
        if approx_dist(target.0 - list[i].x, target.1 - list[i].y) < 0x600 {
            list[i].active = false;
            out.push(StaffEvent::Left { employee: i });
        }
    }
    if list[i].steps == 0 {
        let avoid = w.golfers.get(i ^ 1).filter(|g| g.present).map(|g| (g.x, g.y));
        let e = list[i];
        let d = path_step(w.terrain, target, (e.x, e.y), avoid, w.tick, rng);
        let e = &mut list[i];
        e.dir = d as i8;
        if d == -1 {
            e.dir = heading(angle(delta.0, delta.1));
            e.anim = ANIM_STAND;
            e.wait = rng.below(16) as i16 + 1;
        } else {
            let sx = (e.x >> 6) & 0xf;
            let sy = (e.y >> 6) & 0xf;
            let k = d as usize;
            e.steps = (if DX[k] == 0 {
                if DY[k] == 1 {
                    0x18 - sy
                } else {
                    sy + 8
                }
            } else if DX[k] == 1 {
                0x18 - sx
            } else {
                sx + 8
            }) as i16;
            if d & 1 != 0 && e.steps > 0x10 {
                e.steps = 0x10;
            }
        }
        if list[i].steps == 0 {
            return;
        }
    }
    for j in 0..i {
        if !list[j].active {
            continue;
        }
        let (dx, dy) = (list[j].x - list[i].x, list[j].y - list[i].y);
        if approx_dist(dx, dy) < 0x155 {
            let d8 = octant(dx, dy);
            let mine = list[i].dir as i32;
            if mine == d8 || mine == (d8 + 1) & 7 || mine == (d8 - 1) & 7 {
                list[i].wait = rng.below(4) as i16 + 4;
                list[i].anim = ANIM_STAND;
            }
        }
        if list[i].wait != 0 {
            list[i].dir = heading(angle(dx, dy));
        }
    }
    let e = &mut list[i];
    let (a, b) = (e.x >> 10, e.y >> 10);
    let t = w.terrain;
    let mut speed = if t.inside(a, b) { (6 - layer(t, a, b)).clamp(2, 5) } else { 2 };
    if on_path(t, a, b) {
        speed = 6;
    }
    if e.upgraded {
        speed += 2;
    }
    if e.job == job::OWNER {
        speed += 2;
    }
    if e.going_home {
        speed = 3;
    }
    let k = (e.dir as i32 & 7) as usize;
    let div = if e.dir & 1 != 0 { 4 } else { 3 };
    e.x += (DX[k] * speed * 0x40) / 2 / div;
    e.y += (DY[k] * speed * 0x40) / 2 / div;
    e.steps -= 1;
    if e.steps != 0 && rng.below(4) == 0 {
        e.steps -= 1;
    }
    if e.anim < 7 {
        e.anim = 7;
    } else {
        e.anim += 1;
        if e.anim > 10 {
            e.anim = 7;
        }
    }
}

/// Weeds: every tick on which (tick & 3) <= difficulty, one random tile may sprout a weed. It needs a weed on one side and a
/// tee, green or fairway tile on another (and at most three weeds around), or it is a creek or garden tile (then a one in
/// three chance that falls as the neighbouring weeds grow). Only fairways and rough- or tree-class tiles take weeds.
pub fn spread_weeds(t: &Terrain, tiles: &mut TileState, tick: u32, difficulty: i32, rng: &mut ExeRng, out: &mut Vec<StaffEvent>) {
    if (tick & 3) as i32 > difficulty {
        return;
    }
    let a = rng.below(50);
    let b = rng.below(50);
    if !t.inside(a, b) {
        return;
    }
    let ti = tiles.i(a, b);
    if tiles.flags[ti] & 0xc00 != 0 || footprint(t, a, b) {
        return;
    }
    let (mut mask, mut count) = (0, 0);
    for k in [0usize, 2, 4, 6] {
        let (na, nb) = (a + DX[k], b + DY[k]);
        if !tile_ok(t, na, nb) {
            continue;
        }
        if tiles.flags[tiles.i(na, nb)] & WEEDS != 0 {
            mask |= 1;
            count += 1;
        }
        if TYPES.get(t.type_at(na, nb) as usize).map(|i| i.desert_group <= 0).unwrap_or(false) {
            mask |= 2;
        }
    }
    let mut grow = mask == 3 && count <= 3;
    if tiles.flags[ti] & SEEDS_WEEDS != 0 {
        grow = rng.below(3) <= count;
    }
    if !grow {
        return;
    }
    let ty = t.type_at(a, b);
    if ty == 2 || class(ty) == 4 || class(ty) == 13 {
        tiles.flags[ti] |= WEEDS | WORKED;
        tiles.counter[ti] = 1;
        out.push(StaffEvent::Sound { slot: 0x22, x: a * UNIT + 0x200, y: b * UNIT + 0x200 });
    }
}

/// Weed growth, once per tick (the exe does it while drawing each tile): a tile marked as being worked on advances its counter
/// by one (with chance 1/8 while a weed stands there) and stops being worked on once the counter is past 24. A new weed is
/// therefore growing until then; a grown weed is what golfers complain about and what a Groundskeeper has to work on.
pub fn grow_weeds(tiles: &mut TileState, rng: &mut ExeRng) {
    for i in 0..tiles.flags.len() {
        if tiles.flags[i] & WORKED == 0 {
            continue;
        }
        let c = tiles.counter[i];
        if tiles.flags[i] & WEEDS == 0 || rng.below(8) == 0 {
            tiles.counter[i] = c.wrapping_add(1);
        }
        if c > 0x18 {
            tiles.flags[i] &= !WORKED;
        }
    }
}

/// A grown weed on a tile (one golfers notice).
pub fn grown_weed(tiles: &TileState, a: i32, b: i32) -> bool {
    if a < 0 || b < 0 || a >= tiles.w || b >= tiles.h {
        return false;
    }
    let f = tiles.flags[tiles.i(a, b)];
    f & WEEDS != 0 && f & WORKED == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_the_exe() {
        assert_eq!(approx_dist(3, -4), 5);
        assert_eq!(octant(10, 0), 2);
        assert_eq!(octant(0, -10), 0);
        assert_eq!(octant(-10, 10), 5);
        assert_eq!(heading(angle(0, -100)), 0);
        assert_eq!(heading(angle(100, 0)), 2);
        assert_eq!(heading(angle(0, 100)), 4);
        assert_eq!(heading(angle(-100, 0)), 6);
        assert_eq!(heading(angle(100, 100)), 3);
        assert_eq!(true_dist(3, 4), 5);
    }

    fn flat(w: i32) -> Terrain {
        let mut t = Terrain::demo_course(w, w, 1);
        for v in t.ty.iter_mut() {
            *v = 4;
        }
        for v in t.path_kind.iter_mut() {
            *v = 0;
        }
        t
    }

    #[test]
    fn soda_vendor_walks_to_a_thirsty_golfer_and_sells_a_drink() {
        let t = flat(50);
        let mut tiles = TileState::new(50, 50);
        let mut rng = ExeRng::from_clock(5);
        let mut staff = Vec::new();
        let club = (20, 20);
        let v = hire(&mut staff, 3, false, club, &mut rng);
        let post = staff[v].post.unwrap();
        let mut golfers = vec![StaffGolfer::default(); 2];
        golfers[0] = StaffGolfer {
            present: true,
            x: (post.0 + 2) * UNIT + 512,
            y: (post.1 + 1) * UNIT + 512,
            thirst: 20,
            face: -1,
            ..Default::default()
        };
        let mut out = Vec::new();
        let mut sold = false;
        for tick in 1..4000u32 {
            let mut world =
                World { terrain: &t, tiles: &mut tiles, golfers: &mut golfers, club, clicked: None, tick, difficulty: 1, weed_frames: 8 };
            tick_all(&mut staff, &mut world, &mut rng, &mut out);
            if out.iter().any(|e| matches!(e, StaffEvent::DrinkSold { .. })) {
                sold = true;
                break;
            }
        }
        assert!(sold, "no drink sold; vendor at {:?}", (staff[v].x >> 10, staff[v].y >> 10));
        assert_eq!(golfers[0].thirst, 0);
    }

    fn tick_all(staff: &mut [Employee], w: &mut World, rng: &mut ExeRng, out: &mut Vec<StaffEvent>) {
        tick(staff, w, rng, out);
    }
}
