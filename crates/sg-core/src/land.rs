//! New-game land, as the publisher's golf.exe builds it (docs/PUBLISHER_EXE_NOTES.md, "Properties and new-game land").
//!
//! A new game first deals the sixteen properties out to sixteen "offer slots" at random. The slot decides the acreage and the
//! price; the property decides the theme, the coast and the relief. Picking a property runs the land generator for its slot:
//! random streaks of trees, a creek, a coastline, scattered ponds and rock patches, wildlife spots, the clubhouse, the
//! property's own landmark or free building, a few obstacles that grow with the difficulty, and finally the land outside the
//! purchased acreage. Everything here follows the exe's rules and its random number generator; the code is our own.
//!
//! Map layout follows the exe: tiles are addressed (a, b) with a the first index, 50 x 50 tiles, and corner heights on a 51 x 51
//! grid. `to_terrain` converts to the port's Terrain (x = a, y = b).

use crate::terrain::Terrain;

pub const N: i32 = 50;
const NN: usize = (N * N) as usize;
const HN: usize = 51 * 51;

/// The exe's random number generator: a 32-bit LCG (x * 1103515245 + 12345), seeded with the Windows millisecond clock times 37.
/// A draw takes bits 16..30 as a fraction of 1 and scales it by the (16-bit) range, truncating.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExeRng {
    pub state: u32,
}

impl ExeRng {
    pub fn from_clock(ms: u32) -> Self {
        ExeRng { state: ms.wrapping_mul(37) }
    }
    pub fn below(&mut self, n: i32) -> i32 {
        self.state = self.state.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        let r = (self.state >> 16) & 0x7fff;
        ((r as f64 / 32768.0) * ((n as u32 & 0xffff) as f64)) as i32
    }
}

/// Height noise: a 16 x 16 grid of random values 0..15 that wraps around, read with bilinear interpolation. The exe fills it once
/// at start-up (drawing 18 x 18 values, of which the 16 x 16 corner is used).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Noise {
    g: [[i32; 17]; 17],
}

impl Noise {
    pub fn new(rng: &mut ExeRng) -> Noise {
        let mut src = [[0i32; 18]; 18];
        for row in src.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.below(16);
            }
        }
        let mut g = [[0i32; 17]; 17];
        for r in 0..16 {
            for c in 0..16 {
                g[r][c] = src[r][c];
            }
            g[r][16] = g[r][0];
        }
        g[16] = g[0];
        Noise { g }
    }

    fn sample(&self, x: i32, y: i32) -> i32 {
        let i = (((x - 0x80) >> 8) & 15) as usize;
        let j = (((y - 0x80) >> 8) & 15) as usize;
        let fx = ((x - 0x80) >> 3) & 31;
        let fy = ((y - 0x80) >> 3) & 31;
        let g = &self.g;
        let s = g[i][j + 1] * (32 - fx) * fy + g[i + 1][j] * (32 - fy) * fx + g[i][j] * (32 - fx) * (32 - fy) + g[i + 1][j + 1] * fy * fx;
        s / 32
    }

    /// Two octaves mixed 6:4, scaled to 0..512.
    pub fn field(&self, a: i32, b: i32) -> i32 {
        let n1 = self.sample(a >> 2, b >> 2);
        let n2 = self.sample((a >> 2) * 2, (b >> 2) * 2);
        let v = (n1 * 6 + n2 * 4) * 7;
        clamp_exe((v / 64) >> 1, 0, 0x200)
    }
}

/// The exe's clamp: below lo gives lo; above hi gives hi only when lo <= hi.
fn clamp_exe(v: i32, lo: i32, hi: i32) -> i32 {
    let v = v.max(lo);
    if v > hi && lo <= hi {
        hi
    } else {
        v
    }
}

/// One of the exe's sixteen property records.
#[derive(Clone, Copy, Debug)]
pub struct PropertyRecord {
    pub name: &'static str,
    /// The course's own name (record +0x19), used in the date caption and the messages.
    pub course: &'static str,
    pub bonus: &'static str,
    /// Marker position on the world map art.
    pub map_x: i32,
    pub map_y: i32,
    /// Exe theme order: 0 Parkland, 1 Desert, 2 Tropical, 3 Links.
    pub theme: u8,
    /// 0 inland (owned land is a square), 1 coast (the sea runs along one side), 2 island (the land is ringed by water).
    pub coast: u8,
    /// 0 gentle, 1 rolling, 2 hilly.
    pub relief: u8,
}

#[allow(clippy::too_many_arguments)]
const fn rec(
    name: &'static str,
    course: &'static str,
    bonus: &'static str,
    map_x: i32,
    map_y: i32,
    theme: u8,
    coast: u8,
    relief: u8,
) -> PropertyRecord {
    PropertyRecord { name, course, bonus, map_x, map_y, theme, coast, relief }
}

pub const RECORDS: [PropertyRecord; 16] = [
    rec("Monterey", "Ocean's Edge", "Scenic Cypress", 53, 226, 0, 1, 1),
    rec("San Diego", "Dolphin Coast", "Dolphins", 53, 244, 0, 1, 0),
    rec("Rocky Mtns.", "Jurassic Springs", "Free Hotel", 92, 226, 0, 0, 2),
    rec("Las Vegas", "Ace in the Hole", "Fun, Fun, Fun", 73, 241, 1, 0, 0),
    rec("Phoenix", "Coyote Flats", "Free Spa", 70, 262, 1, 0, 0),
    rec("Hawaii", "Flamingo Shores", "Scenic Waterfall", 26, 327, 2, 1, 1),
    rec("Oahu", "Island Palms", "Japanese Garden", 23, 311, 2, 2, 0),
    rec("Nova Scotia", "Windy Point", "Scenic Lighthouse", 158, 250, 3, 1, 1),
    rec("Northeast", "Ravenwood Farms", "Civil War Battlefield", 138, 254, 0, 0, 1),
    rec("Carolina", "Christmas Pines", "Free Putting Green", 133, 272, 0, 0, 0),
    rec("Ireland", "County Kincaide", "Leprechauns", 262, 232, 3, 0, 2),
    rec("Scotland", "Harold's Keep", "Free Castle", 266, 222, 3, 1, 0),
    rec("Wales", "Thistle Runes", "Stonehenge", 272, 239, 3, 2, 1),
    rec("Spain", "Sangria Bay", "Scenic Vineyards", 275, 270, 1, 1, 1),
    rec("Florida", "Ocean Grove", "Free Pro Shop", 125, 298, 2, 1, 0),
    rec("Jamaica", "Scorpion Cove", "Scenic Statues", 129, 342, 2, 0, 1),
];

/// Price of an offer slot, in the exe's money units (one unit is 100 of the game's money).
pub const SLOT_PRICE_UNITS: [i32; 16] = [500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000];

/// One offer slot: which property it holds and the acreage the deal comes with.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub property: usize,
    pub acres: i32,
}

impl Slot {
    pub fn record(&self) -> &'static PropertyRecord {
        &RECORDS[self.property]
    }
}

/// Deals the properties out to the sixteen slots. Slots 12..15 (the dearest) get four different themes; slot 0 (the cheapest)
/// always gets a Parkland property; the rest are random. Acres are (slot + 4) x 10, plus 10 (slots 0..3) or 20 (later slots)
/// for an inland property, minus the same for an island; sandbox games get 250 acres everywhere.
pub fn deal_offer(rng: &mut ExeRng, sandbox: bool) -> [Slot; 16] {
    let mut slot: [Option<usize>; 16] = [None; 16];
    let acres_for = |i: i32, p: usize| -> i32 {
        if sandbox {
            return 250;
        }
        let bonus = if i > 3 { 20 } else { 10 };
        let base = (i + 4) * 10;
        match RECORDS[p].coast {
            0 => base + bonus,
            2 => base - bonus,
            _ => base,
        }
    };
    let mut out = [Slot { property: 0, acres: 0 }; 16];
    // Slots 12..15: one property per theme.
    let mut i = 12;
    while i < 16 {
        let p = rng.below(16) as usize;
        if (12..i).any(|j| RECORDS[slot[j].unwrap()].theme == RECORDS[p].theme) {
            continue;
        }
        slot[i] = Some(p);
        out[i] = Slot { property: p, acres: acres_for(i as i32, p) };
        i += 1;
    }
    // Slots 0..11: properties not dealt yet; slot 0 must be Parkland.
    let mut i = 0;
    while i < 12 {
        let p = rng.below(16) as usize;
        let taken = slot.contains(&Some(p));
        if taken || (i == 0 && RECORDS[p].theme != 0) {
            continue;
        }
        slot[i] = Some(p);
        out[i] = Slot { property: p, acres: acres_for(i as i32, p) };
        i += 1;
    }
    out
}

/// Per-type table the exe keeps for its 23 tile types (copied at start-up from a static table in the exe).
#[derive(Clone, Copy, Debug)]
pub struct TypeInfo {
    pub name: &'static str,
    /// Byte +0x22: types with a positive value collapse into class 4 for border drawing in the Desert theme (types 5 and 7 excepted).
    pub desert_group: i8,
    /// Byte +0x24: what a tile of this type adds to the clearing cost of a building footprint.
    pub clear_cost: u8,
    /// Byte +0x25: copied to a per-tile layer whenever a tile is set (its use is not traced yet).
    pub layer: u8,
    /// Byte +0x26: the class handed to Terrain.dll for border drawing (tiles of one class blend).
    pub class: u8,
}

const fn ti(name: &'static str, desert_group: i8, clear_cost: u8, layer: u8, class: u8) -> TypeInfo {
    TypeInfo { name, desert_group, clear_cost, layer, class }
}

pub const TYPES: [TypeInfo; 23] = [
    ti("tees", 0, 0, 2, 0),
    ti("green", -1, 0, 1, 1),
    ti("fairway", 0, 0, 1, 2),
    ti("firm fairway", 0, 0, 1, 2),
    ti("rough", 1, 0, 2, 4),
    ti("deep rough", 2, 0, 3, 4),
    ti("mound", 2, 0, 3, 4),
    ti("sand trap", 3, 0, 4, 7),
    ti("waste bunker", 2, 0, 3, 8),
    ti("pot bunker", 5, 0, 5, 7),
    ti("ravine", 5, 0, 3, 4),
    ti("brush", 3, 0, 3, 4),
    ti("rocks", 3, 5, 4, 4),
    ti("tree", 4, 5, 4, 13),
    ti("pine tree", 4, 5, 4, 13),
    ti("palm tree", 4, 5, 4, 13),
    ti("elm tree", 4, 10, 4, 13),
    ti("water", 8, 10, 16, 17),
    ti("wetlands", 3, 50, 4, 4),
    ti("marsh", 3, 100, 4, 4),
    ti("out of bounds", 8, 0, 12, 18),
    ti("building", 4, 0, 4, 18),
    ti("building", 4, 0, 4, 18),
];

pub const T_TEE: u8 = 0;
pub const T_GREEN: u8 = 1;
pub const T_ROUGH: u8 = 4;
pub const T_DEEP_ROUGH: u8 = 5;
pub const T_WASTE: u8 = 8;
pub const T_RAVINE: u8 = 10;
pub const T_BRUSH: u8 = 11;
pub const T_ROCKS: u8 = 12;
pub const T_TREE: u8 = 13;
pub const T_PINE: u8 = 14;
pub const T_PALM: u8 = 15;
pub const T_ELM: u8 = 16;
pub const T_WATER: u8 = 17;
pub const T_WETLANDS: u8 = 18;
pub const T_MARSH: u8 = 19;
pub const T_OUT: u8 = 20;
pub const T_HOME_SITE: u8 = 21;
pub const T_BUILDING: u8 = 22;

/// Border class of a tile type as the exe hands it to Terrain.dll, with the Desert override.
pub fn border_class(ty: u8, desert: bool) -> u8 {
    let Some(t) = TYPES.get(ty as usize) else { return ty };
    if desert && t.desert_group > 0 && ty != 5 && ty != 7 {
        4
    } else {
        t.class
    }
}

/// Buildings and objects: (name, footprint edge in tiles, price in money units), by the exe's object kind.
pub const BUILDINGS: [(&str, i32, i32); 20] = [
    ("Pathway", 1, 1),
    ("Benches", 1, 2),
    ("Flower Bed", 1, 5),
    ("Ball Washer", 1, 50),
    ("Landmark", 1, 15),
    ("Home Site", 2, 10),
    ("Putting Green", 3, 100),
    ("Snack Bar", 2, 150),
    ("Pro Shop", 2, 200),
    ("Swim Club", 3, 300),
    ("Driving Range", 5, 250),
    ("Cart Garage", 2, 400),
    ("Marina", 2, 1000),
    ("Resort Hotel", 4, 2500),
    ("Airstrip", 6, 5000),
    ("Clubhouse", 4, 200),
    ("Willow Tree", 1, 25),
    ("TV Tower", 1, 10),
    ("TV Booth", 1, 10),
    ("Scenic Bridge", 1, 100),
];
/// No undo record on a tile.
pub const UNDO_NONE: u8 = 0xff;
pub const K_PATH: i32 = 0;
pub const K_BENCH: i32 = 1;
pub const K_FLOWERS: i32 = 2;
pub const K_WILLOW: i32 = 16;
pub const K_BRIDGE: i32 = 19;
pub const K_TV_TOWER: i32 = 17;
pub const K_TV_BOOTH: i32 = 18;
pub const K_LANDMARK: i32 = 4;
pub const K_HOME_SITE: i32 = 5;
pub const K_PUTTING_GREEN: i32 = 6;
pub const K_DRIVING_RANGE: i32 = 10;
pub const K_SNACK_BAR: i32 = 7;
pub const K_MARINA: i32 = 12;
pub const K_CLUBHOUSE: i32 = 15;

/// Tile flag bits the generator uses.
pub mod flag {
    /// A path is laid on the tile.
    pub const PATH: u16 = 0x20;
    /// The path (or building) is joined to the clubhouse.
    pub const JOINED: u16 = 0x40;
    /// A wildlife or scenery spot.
    pub const SPOT: u16 = 0x100;
    /// Second path style bit, set on alternate tiles around some landmarks.
    pub const PATH_ALT: u16 = 0x200;
    /// Part of a building or object footprint.
    pub const FOOTPRINT: u16 = 0x400;
    /// Creek tiles (cleared again before the generator finishes) and garden borders.
    pub const CREEK: u16 = 0x1000;
    /// Set on the 48th tile of a tree streak.
    pub const STREAK_48: u16 = 0x2000;
    /// Holds a difficulty obstacle.
    pub const OBSTACLE: u16 = 0x8000;
}

/// An object on the map (the exe's 16-byte object record): a building, a landmark or an obstacle.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub kind: i32,
    pub a: i32,
    pub b: i32,
    /// Facing 0..3.
    pub dir: u8,
    pub flags: u8,
    /// Landmark look (what the landmark is), for kind 4; building level for kinds 6 and up; a home site's owner.
    pub sub: i32,
    /// A home site's smoothed lot value (+0xc), which sizes the house drawn on it.
    pub val: i32,
}

/// Generated land in the exe's layout.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Land {
    pub slot_index: usize,
    pub slot: Slot,
    pub difficulty: i32,
    pub ty: Vec<u8>,
    pub flags: Vec<u16>,
    pub layer: Vec<u8>,
    /// Per tile: water depth 0..2 for water, object index (0xff = none) for footprints.
    pub var: Vec<u8>,
    /// Corner heights, 51 x 51, sea level 3, at most 15.
    pub height: Vec<u8>,
    /// The map before the land outside the purchase was marked; buying land restores tiles from here.
    pub original: Vec<u8>,
    pub objects: Vec<Object>,
    /// Clubhouse anchor (its footprint is 4 x 4 tiles from here).
    pub clubhouse: (i32, i32),
    /// Where the first employee starts (one tile in from the clubhouse anchor).
    pub staff_start: (i32, i32),
    /// Map point the exe stores with the clubhouse: (anchor a + 6, anchor b - 4).
    pub view: (i32, i32),
    /// Width of the unowned border.
    pub margin: i32,
    hill: i32,
    /// Per tile undo record (0x5830b8): 0xff none, below 0x7d the tile type before a terrain change, 0x80 | kind for an item
    /// placed on the tile; and the amount (units) an undo gives back (0x59c090).
    pub undo: Vec<u8>,
    pub refund: Vec<i8>,
    /// Growth counter per tile (0x578804; flower beds and willows grow, as weeds do).
    pub growth: Vec<u8>,
}

const DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

fn idx(a: i32, b: i32) -> usize {
    (a * N + b) as usize
}

impl Land {
    pub fn tile(&self, a: i32, b: i32) -> u8 {
        self.ty[idx(a, b)]
    }

    /// Tile type at a flat index; reads past the map count as land (the exe reads neighbouring memory there).
    fn ty_at(&self, i: i32) -> u8 {
        if (0..NN as i32).contains(&i) {
            self.ty[i as usize]
        } else {
            0xff
        }
    }

    fn set_ty(&mut self, i: i32, t: u8) {
        if (0..NN as i32).contains(&i) {
            self.ty[i as usize] = t;
            self.layer[i as usize] = TYPES[t as usize].layer;
        }
    }

    fn h(&self, i: i32) -> u8 {
        if (0..HN as i32).contains(&i) {
            self.height[i as usize]
        } else {
            0
        }
    }

    fn set_h(&mut self, i: i32, v: u8) {
        if (0..HN as i32).contains(&i) {
            self.height[i as usize] = v;
        }
    }

    fn flags_or(&mut self, i: i32, f: u16) {
        if (0..NN as i32).contains(&i) {
            self.flags[i as usize] |= f;
        }
    }

    /// 1 for tiles off the map or outside the owned land, else 0.
    fn blocked(&self, a: i32, b: i32) -> bool {
        !(0..N).contains(&a) || !(0..N).contains(&b) || self.tile(a, b) == T_OUT
    }

    /// Corner height while the land is being made: from the noise, by relief and difficulty. Water corners and corners of
    /// wetlands and marsh sit at sea level.
    fn noise_height(&self, noise: &Noise, a: i32, b: i32) -> u8 {
        if !(0..N).contains(&a) || !(0..N).contains(&b) {
            return 3;
        }
        let here = self.tile(a, b);
        let next = self.ty_at(idx(a, b) as i32 + N);
        if here == T_WATER || next == T_WATER || here == T_WETLANDS || here == T_MARSH {
            return 3;
        }
        let rec = self.slot.record();
        let mut n = if rec.relief == 2 { noise.field(a << 7, b << 7) } else { noise.field(a << 6, b << 6) };
        if rec.theme == 1 && b < 16 {
            n -= (16 - b) * self.hill / 6;
        }
        clamp_exe(n / self.hill + 1, 3 + (self.difficulty != 0) as i32, 15) as u8
    }

    /// Corner height once the game runs: sea level off the map and deep in the unowned land, else the stored height.
    pub fn corner(&self, a: i32, b: i32) -> u8 {
        if !(0..N).contains(&a) || !(0..N).contains(&b) {
            return 3;
        }
        if self.tile(a, b) == T_OUT && self.blocked(a - 1, b) && self.blocked(a, b + 1) {
            return 3;
        }
        self.height[(a * 51 + b) as usize]
    }

    fn object_at(&self, i: usize) -> Option<&Object> {
        self.objects.get(self.var[i] as usize)
    }

    /// Footprint test for an object of `kind` and edge `size` at (a, b), with a one-tile ring around it. Returns the clearing
    /// cost, or None when the spot is not allowed.
    pub fn fits(&self, a: i32, b: i32, size: i32, kind: i32, theme: u8) -> Option<i32> {
        let mut cost = 0;
        let mut dry_ring = false;
        if size >= -1 {
            for r in -1..=size {
                for c in -1..=size {
                    let (ta, tb) = (a + r, b + c);
                    let i = ta * N + tb;
                    let ring = r == -1 || c == -1 || r == size || c == size;
                    if ring {
                        if self.ty_at(i) != T_WATER {
                            dry_ring = true;
                        }
                        continue;
                    }
                    if self.blocked(ta, tb) {
                        return None;
                    }
                    let t = self.ty_at(i);
                    let f = self.flags[i as usize];
                    if kind == K_MARINA && (theme == 0 || theme == 2) && t != T_WATER {
                        return None;
                    }
                    if kind != 0 {
                        if t == T_HOME_SITE || t == 0 || f & 0x8080 != 0 {
                            return None;
                        }
                        if (t == T_BUILDING || f & flag::FOOTPRINT != 0) && self.object_at(i as usize).map(|o| o.kind) != Some(kind) {
                            return None;
                        }
                    }
                    if TYPES[t as usize].class == 13
                        || t == T_ROCKS
                        || (t == T_WATER && kind != K_MARINA)
                        || t == T_WETLANDS
                        || t == T_MARSH
                    {
                        cost += TYPES[t as usize].clear_cost as i32;
                    }
                    if f & 0x8000 != 0 {
                        return None;
                    }
                }
            }
            if dry_ring {
                return Some(cost);
            }
        }
        if kind != 0 && kind != 4 {
            return None;
        }
        Some(cost)
    }

    /// Puts down an object: flattens its corners to the anchor's height, turns its tiles into building tiles (a putting green
    /// keeps green tiles, a driving range rough, a marina water) and records it. Returns the object's index.
    pub fn place(&mut self, rng: &mut ExeRng, a: i32, b: i32, kind: i32, fl: i32, theme: u8) -> usize {
        self.place_sized(rng, a, b, kind, fl, theme, (fl == -2) as i32)
    }

    /// Places an object `extra` tiles larger than its kind's edge (an upgraded building is one larger).
    #[allow(clippy::too_many_arguments)]
    pub fn place_sized(&mut self, rng: &mut ExeRng, a: i32, b: i32, kind: i32, fl: i32, theme: u8, extra: i32) -> usize {
        let size = BUILDINGS[kind as usize].1 + extra;
        let anchor_h = self.h(a * 51 + b);
        for c in 0..size {
            for k in 0..size {
                if k < size - 1 || c != 0 {
                    self.set_h((c + a) * 51 + b + k, anchor_h);
                }
                let i = (b + k) + (c + a) * N;
                let mut t = if kind != K_HOME_SITE { T_BUILDING } else { T_HOME_SITE };
                let edge = c != 0 || k != 0;
                if kind == K_PUTTING_GREEN && edge {
                    t = T_GREEN;
                } else if kind == K_DRIVING_RANGE && edge {
                    t = if c == size - 1 { T_DEEP_ROUGH } else { T_ROUGH };
                } else if kind == K_MARINA && (theme == 0 || theme == 2) && edge {
                    t = T_WATER;
                }
                if (0..NN as i32).contains(&i) {
                    self.ty[i as usize] = t;
                    self.layer[i as usize] = TYPES[T_BUILDING as usize].layer;
                    let f = self.flags[i as usize] & 0xecff;
                    self.flags[i as usize] = if fl > 0 { f | fl as u16 } else { f };
                    self.var[i as usize] = 0xff;
                }
            }
        }
        if kind == K_DRIVING_RANGE {
            let span = size - 2;
            for mark in [T_GREEN, T_GREEN, T_GREEN, 7] {
                let r1 = rng.below(span);
                let r2 = rng.below(span);
                let i = b + r1 + (r2 + a) * N + 51;
                if (0..NN as i32).contains(&i) {
                    self.ty[i as usize] = mark;
                }
            }
        } else if kind == K_CLUBHOUSE {
            self.staff_start = (a + 1, b + 1);
        }
        if fl < 0 {
            return 0;
        }
        self.flags_or(a * N + b, kind as u16 + 1);
        // The exe reuses the first free object record.
        let n = match self.objects.iter().position(|o| o.kind == -1) {
            Some(i) => {
                self.objects[i] = Object { kind, a, b, dir: 0, flags: 0, sub: 0, val: 0 };
                i
            }
            None => {
                self.objects.push(Object { kind, a, b, dir: 0, flags: 0, sub: 0, val: 0 });
                self.objects.len() - 1
            }
        };
        let size = BUILDINGS[kind as usize].1;
        for r in 0..size {
            for c in 0..size {
                let i = (a + r) * N + b + c;
                if (0..NN as i32).contains(&i) {
                    self.var[i as usize] = n as u8;
                    self.flags[i as usize] = self.flags[i as usize] & 0xfeff | flag::FOOTPRINT;
                }
            }
        }
        n
    }

    /// Water depth: every water tile starts deep (2); next to land it is shallow (0); next to shallow water it is middling (1).
    fn water_depth(&mut self) {
        for i in 0..NN {
            if TYPES[self.ty[i] as usize].class == 17 {
                self.var[i] = 2;
            }
        }
        loop {
            let mut changed = false;
            for a in 0..N {
                for b in 0..N {
                    let i = idx(a, b);
                    if TYPES[self.ty[i] as usize].class != 17 {
                        continue;
                    }
                    let mut v = self.var[i];
                    for k in 0..8 {
                        let (na, nb) = (a + DX[k], b + DY[k]);
                        if self.blocked(na, nb) {
                            continue;
                        }
                        let j = idx(na, nb);
                        if TYPES[self.ty[j] as usize].class != 17 {
                            v = 0;
                        }
                        if self.var[j] == 0 && v != 0 {
                            v = 1;
                        }
                    }
                    if v != self.var[i] {
                        self.var[i] = v;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Marks a tile as outside the owned land: out of bounds, or water around an island.
    fn unown(&mut self, a: i32, b: i32) {
        let i = idx(a, b);
        if self.slot.record().coast != 2 {
            self.ty[i] = T_OUT;
        } else {
            self.ty[i] = T_WATER;
            self.flags[i] &= 0xfcdf;
        }
    }
}

/// Upper bound on retries in the generator's "draw until it fits" loops; the exe loops without a bound.
const TRIES: i32 = 100_000;

/// Builds the land for an offer slot, as the exe does when a property is bought.
pub fn generate(slot_index: usize, slot: Slot, difficulty: i32, sandbox: bool, rng: &mut ExeRng, noise: &Noise) -> Land {
    let rec = *slot.record();
    let theme = rec.theme;
    let prop = slot.property as i32;
    let desert = theme == 1;
    let si = slot_index as i32;

    // Height scale: bigger means flatter.
    let mut hill = [48, 32, 16][rec.relief.min(2) as usize];
    match si / 4 {
        0 => hill += hill / 2,
        1 => hill += hill / 4,
        3 => hill -= hill / 4,
        _ => {}
    }
    if sandbox {
        hill += 16;
    }
    // Ground: rough; the dearer slots start from deep rough or brush (rocks in the desert).
    let mut base = T_ROUGH;
    if !sandbox {
        if si > 7 {
            base = if desert { T_ROCKS } else { T_DEEP_ROUGH };
        }
        if si > 11 {
            base = if desert { T_ROCKS } else { T_BRUSH };
        }
    }
    let mut land = Land {
        slot_index,
        slot,
        difficulty,
        ty: vec![base; NN],
        flags: vec![0; NN],
        layer: vec![TYPES[base as usize].layer; NN],
        var: vec![0; NN],
        height: vec![0; HN],
        original: Vec::new(),
        objects: Vec::new(),
        clubhouse: (-1, -1),
        staff_start: (-1, -1),
        view: (-1, -1),
        margin: 0,
        hill,
        undo: vec![UNDO_NONE; NN],
        refund: vec![0; NN],
        growth: vec![0; NN],
    };

    // Streaks of trees (and rocks, brush or deep rough, by theme), each a random walk. Up to 1250 tiles in all.
    let mut limit = 5000;
    let mut total = 0;
    let mut n = 0;
    loop {
        let mut a = rng.below(50);
        let mut b = rng.below(50);
        let odd = n & 1 == 1;
        let t: u8 = match theme {
            0 => {
                if prop == 9 || b >= 25 {
                    T_PINE
                } else {
                    T_TREE
                }
            }
            1 => {
                if odd {
                    T_ROCKS
                } else {
                    T_PINE
                }
            }
            2 => {
                if !odd {
                    T_PINE
                } else if n & 2 != 0 {
                    T_TREE
                } else {
                    T_ROCKS
                }
            }
            _ => {
                if odd {
                    T_BRUSH
                } else {
                    T_DEEP_ROUGH
                }
            }
        };
        let mut len = 0;
        loop {
            if a < 0 || b < 0 || a >= N || b >= N || len >= limit / 128 {
                break;
            }
            let i = idx(a, b);
            len += 1;
            land.ty[i] = t;
            if len == 48 {
                land.flags[i] |= flag::STREAK_48;
            }
            land.layer[i] = TYPES[t as usize].layer;
            if rng.below(64) == 0 {
                land.flags[i] |= flag::SPOT;
            } else {
                land.flags[i] &= !flag::SPOT;
            }
            a += rng.below(3) - 1;
            b += rng.below(3) - 1;
            if a < 0 {
                break;
            }
        }
        total += len;
        if total > 1250 {
            break;
        }
        limit += 2500;
        n += 1;
        if limit >= 45000 {
            break;
        }
    }

    // A creek from the far edge (b = 49) that runs downhill, with banks of waste bunker or rocks (ravine in the desert).
    if rec.coast != 2 {
        let creek_t = if desert { T_BRUSH } else { T_WATER };
        let mut a = 50 / 3 + rng.below(16);
        let mut b = 49;
        let mut prev_dir: i32 = 0;
        let mut dir: i32 = 0;
        while (0..N).contains(&a) {
            let i = a * N + b;
            land.set_ty(i, creek_t);
            land.flags[i as usize] = land.flags[i as usize] & 0xfeff | flag::CREEK;
            if rng.below(64) == 0 {
                land.flags_or(i, flag::SPOT);
            }
            let mut nb = b + 1;
            let mut best = 999;
            let side = a + if rng.below(2) != 0 { 1 } else { -1 };
            let bank = side * N + b + 1;
            let mut na = side;
            if land.ty_at(bank) != T_WATER && land.noise_height(noise, side, b + 1) == 3 {
                let v = if desert { T_RAVINE } else { (!((side + b + 1) * 2) & 4 | 8) as u8 };
                land.set_ty(bank, v);
            }
            let mut d: i32 = -2;
            while d < 3 {
                let k = (d & 7) as usize;
                let ca = DX[k] + a;
                let cb = DY[k] + b;
                let hgt = noise.field(ca * 0x80, cb * 0x80);
                if hgt < best {
                    dir = d;
                    na = ca;
                    best = hgt;
                    nb = cb;
                }
                d += 2;
            }
            if dir == -prev_dir {
                nb = b - 1;
                na = a;
            }
            prev_dir = dir;
            if land.ty_at(na * N + nb) == T_WATER || nb < 0 {
                break;
            }
            a = na;
            b = nb;
        }
    }

    // The sea along the b = 0 side of a coastal property, 2..12 tiles wide; the first tile of each row is out of bounds.
    if rec.coast == 1 {
        let mut w = 5;
        for a in 0..N {
            for b in 0..w {
                let i = idx(a, b);
                if land.ty[i] != T_WATER {
                    let t = if theme == 4 { T_ROCKS } else { T_WATER };
                    land.ty[i] = t;
                    land.layer[i] = TYPES[t as usize].layer;
                    land.flags[i] &= !flag::SPOT;
                    if b == w - 1 && rng.below(16) == 0 {
                        land.flags[i] |= flag::SPOT;
                    }
                }
                if b == 0 {
                    land.ty[idx(a, 0)] = T_OUT;
                }
            }
            w += rng.below(5) - 2;
            if w < 2 {
                w = 2;
            } else if w > 12 {
                w -= 1;
            }
        }
    }

    for f in land.flags.iter_mut() {
        *f &= !flag::CREEK;
    }
    for a in 0..51 {
        for b in 0..51 {
            let v = land.noise_height(noise, a, b);
            land.height[(a * 51 + b) as usize] = v;
        }
    }

    // 24 patches on a 4 x 6 grid: ponds in the low ground, rocks (or sand, by theme) on the hills; the first is a wetland, and
    // on the harder difficulties more of them are.
    for n in 0..24 {
        let mut a = rng.below(4) + 4 + (n % 4) * 50 / 4;
        let mut b = rng.below(4) + 4 + (n / 4 * 50) / 4;
        let high = land.h(b + a * 51) > 3;
        let mut t: u8 = match theme {
            0 => {
                if high {
                    T_ROCKS
                } else {
                    T_WATER
                }
            }
            1 => {
                if high {
                    T_ROCKS
                } else {
                    T_PALM
                }
            }
            2 => {
                if high {
                    T_PALM
                } else {
                    T_WATER
                }
            }
            _ => {
                if high {
                    T_PALM
                } else {
                    T_ROCKS
                }
            }
        };
        if (difficulty > 1 && rng.below(8 / (difficulty - 1)) == 0) || n == 0 {
            t = T_WETLANDS;
        }
        if prop == 2 && (t == T_WASTE || t == T_WETLANDS) {
            t = T_ROCKS;
        }
        let mut count = 0;
        loop {
            let i = b + a * N;
            let protected = (0..NN as i32).contains(&i) && land.flags[i as usize] & flag::CREEK != 0;
            if !protected {
                if (0..NN as i32).contains(&i) {
                    land.ty[i as usize] = t;
                }
                if rng.below(8) == 0 && t == T_WETLANDS {
                    land.flags_or(i, flag::SPOT);
                }
                if (0..NN as i32).contains(&i) {
                    land.layer[i as usize] = TYPES[land.ty[i as usize] as usize].layer;
                }
                if t == T_WATER {
                    land.set_h(b + a * 51, 3);
                }
            }
            if rng.below(2) == 0 {
                b += if rng.below(2) != 0 { 1 } else { -1 };
            } else {
                a += if rng.below(2) != 0 { 1 } else { -1 };
            }
            if !((0..N).contains(&a) && (0..N).contains(&b)) {
                break;
            }
            count += 1;
            if count >= 5 && rng.below(16) == 0 {
                break;
            }
        }
    }

    // Sixteen spots that the next step clears again (the draws still happen), then elm trees become plain trees.
    for _ in 0..16 {
        let r1 = rng.below(42);
        let r2 = rng.below(42);
        land.flags_or(r1 + r2 * N + 204, flag::SPOT);
    }
    for i in 0..NN {
        land.flags[i] &= !flag::SPOT;
        if land.ty[i] == T_ELM {
            land.ty[i] = T_TREE;
        }
    }

    // Wildlife spots: (4 - difficulty) x 9, not on buildings, rough, wetlands, ravine/brush, or water near the b = 0 edge.
    let skip = if desert { T_BRUSH } else { T_RAVINE };
    let mut placed = 0;
    let mut tries = 0;
    while placed < (4 - difficulty) * 9 && tries < TRIES {
        tries += 1;
        let a = rng.below(46) + 2;
        let b = rng.below(46) + 2;
        if land.blocked(a, b) {
            continue;
        }
        let i = idx(a, b);
        let t = land.ty[i];
        if t != T_HOME_SITE && t != T_BUILDING && t != T_ROUGH && t != T_WETLANDS && t != skip && (t != T_WATER || b > 6) {
            land.flags[i] |= flag::SPOT;
            placed += 1;
        }
    }

    // Around water and wetlands, a tile edge whose two corners are both above sea level is pulled down (from 4) or nudged.
    const RA: [i32; 5] = [0, 1, 1, 0, 0];
    const RB: [i32; 5] = [-1, -1, 0, 0, -1];
    for a in 0..51 {
        for b in 0..51 {
            let t = land.ty_at(a * N + b);
            if t != T_WATER && t != T_WETLANDS {
                continue;
            }
            for k in 0..4 {
                let va = RA[k] + a;
                let vb = RB[k] + b;
                if !((0..N).contains(&va) && (0..N).contains(&vb)) {
                    continue;
                }
                let nbr = land.ty_at(b + DY[2 * k] + (DX[2 * k] + a) * N);
                if nbr == T_WATER {
                    continue;
                }
                let vi = va * 51 + vb;
                let v = land.h(vi);
                let w = land.h((RA[k + 1] + a) * 51 + RB[k + 1] + b);
                if v > 3 && v == w {
                    if v == 4 {
                        land.set_h(vi, 3);
                    } else {
                        let up = rng.below(2) != 0;
                        land.set_h(vi, if up { v + 1 } else { v - 1 });
                    }
                }
            }
        }
    }

    // The clubhouse: a random spot in the middle of the map on plain ground where its 4 x 4 footprint fits.
    let csize = BUILDINGS[K_CLUBHOUSE as usize].1;
    let (mut ca, mut cb, mut ra, mut rb);
    let mut tries = 0;
    loop {
        loop {
            ra = rng.below(17);
            ca = ra + 15;
            rb = rng.below(17);
            cb = rb + 15;
            tries += 1;
            if land.ty[idx(ca, cb)] == base || tries > TRIES {
                break;
            }
        }
        if land.fits(ca, cb, csize, K_CLUBHOUSE, theme).is_some() || tries > TRIES {
            break;
        }
    }
    let club = land.place(rng, ca, cb, K_CLUBHOUSE, 0x60, theme);
    land.objects[club].flags |= 0x40;
    land.view = (ra + 0x15, rb + 0xb);
    land.clubhouse = (ca, cb);
    let ci = idx(ca, cb);
    land.flags[ci] &= !flag::PATH;

    // The property's own feature.
    let free_building = match prop {
        2 => Some(0xd),
        4 => Some(9),
        9 => Some(6),
        11 => Some(0xe),
        14 => Some(8),
        _ => None,
    };
    match prop {
        0 | 10 => {
            let mut count = 0;
            let mut tries = 0;
            while count < 16 && tries < TRIES {
                tries += 1;
                let a = rng.below(46) + 2;
                let b = rng.below(46) + 2;
                if land.blocked(a, b) {
                    continue;
                }
                let i = idx(a, b);
                let t = land.ty[i];
                if t != T_HOME_SITE && t != T_BUILDING && t != T_WATER && land.flags[i] & flag::FOOTPRINT == 0 {
                    land.flags[i] |= flag::SPOT;
                    if prop == 0 {
                        land.ty[i] = T_BRUSH;
                    } else {
                        land.ty[i] = T_ROUGH;
                    }
                    count += 1;
                }
            }
        }
        1 => {
            for _ in 0..(8 - difficulty).max(0) {
                let mut i;
                let mut tries = 0;
                loop {
                    let r1 = rng.below(30);
                    let r2 = rng.below(30);
                    i = r2 + (r1 + 10) * N;
                    tries += 1;
                    if land.ty[i as usize] == T_WATER || tries > TRIES {
                        break;
                    }
                }
                land.flags[i as usize] |= flag::SPOT;
            }
        }
        5 | 6 | 8 | 12 | 13 | 15 => {
            let mut tries = 0;
            loop {
                let r1 = rng.below(40);
                let r2 = rng.below(40);
                tries += 1;
                if land.fits(r1 + 3, r2 + 3, 5, K_CLUBHOUSE, theme).is_some() || tries > TRIES {
                    break;
                }
            }
            for n in 0..(8 - difficulty).max(0) {
                rng.below(7);
                rng.below(7);
                let r1 = rng.below(40);
                let a = r1 + 5;
                let r2 = rng.below(40);
                let b = r2 + 5;
                if land.fits(r1 + 4, r2 + 4, 3, 7, theme).is_none() || land.ty[idx(a, b)] == T_WATER {
                    continue;
                }
                landmark(&mut land, rng, prop, n, a, b, theme);
            }
        }
        7 => {
            for n in 0..2 {
                let r = rng.below(10);
                let a = if n == 0 { 25 - r } else { 25 + r };
                let mut b = 0;
                while (land.ty[idx(a, b)] == T_WATER || land.ty[idx(a, b)] == T_OUT) && b < 48 {
                    b += 1;
                }
                let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
                land.objects[o].sub = if rng.below(2) != 0 { 7 } else { 13 };
                let hi = (b + a * 51) as usize;
                land.height[hi] = land.height[hi].wrapping_add(2);
            }
        }
        _ => {}
    }
    if let Some(kind) = free_building {
        // Beside the clubhouse, with a short path to it.
        if !land.blocked(ca, rb + 20) {
            land.place(rng, ra + 14, rb + 19, kind, 0, theme);
            for k in 2..=4 {
                land.flags_or(ci as i32 + k, flag::PATH);
            }
        } else {
            land.place(rng, ra + 14, rb + 10, kind, 0, theme);
            for k in 1..=3 {
                land.flags_or(ci as i32 - k, flag::PATH);
            }
        }
        if prop == 4 || prop == 3 {
            let (mut a, mut b);
            let mut tries = 0;
            loop {
                a = rng.below(40) + 5;
                b = rng.below(40) + 5;
                tries += 1;
                if land.fits(a, b, 2, 4, theme).is_some() || tries > TRIES {
                    break;
                }
            }
            let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
            land.objects[o].sub = 0xe;
        }
    }

    // Obstacles, one per difficulty step, on plain ground; ringed with brush in the desert.
    for _ in 0..difficulty.max(0) {
        let (mut a, mut b, mut i);
        let mut tries = 0;
        loop {
            a = rng.below(39) + 5;
            b = rng.below(39) + 5;
            i = idx(a, b);
            tries += 1;
            if land.ty[i] == base || tries > TRIES {
                break;
            }
        }
        let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
        land.objects[o].sub = match theme {
            0 => 0x10,
            1 => 0x12,
            _ => 0x11,
        };
        let dir = rng.below(4) as u8;
        land.flags[i] |= flag::OBSTACLE;
        land.objects[o].dir = dir;
        if desert {
            for k in 0..8 {
                let i = b + DY[k] + (DX[k] + a) * N;
                if (0..NN as i32).contains(&i) {
                    land.ty[i as usize] = T_BRUSH;
                }
            }
        }
    }

    // The owned land: a square of side 2r where 4r^2 tiles are about acres x 10; the rest is out of bounds.
    let acres = slot.acres & 0xff;
    let mut r = 25;
    let mut m = 0;
    loop {
        m += 1;
        r -= 1;
        if acres * 10 >= r * r * 4 {
            break;
        }
    }
    land.margin = m;
    land.original = land.ty.clone();
    if rec.coast != 2 {
        for i in 0..m {
            for j in 0..N {
                land.unown(i, j);
                if rec.coast == 0 {
                    land.unown(j, i);
                }
                land.unown(j, 49 - i);
                land.unown(49 - i, j);
            }
        }
    } else {
        for row in 0..N {
            let s = rng.below(3);
            let w = clamp_exe((row - 25).abs(), 8, 50) + (m - 1) - 11 + s;
            for j in 0..w.max(0) {
                if j >= N {
                    break;
                }
                land.unown(j, row);
                land.unown(row, j);
                land.unown(row, 49 - j);
                land.unown(49 - j, row);
            }
        }
    }
    land.water_depth();
    land
}

/// A property landmark, placed (8 - difficulty) times for Hawaii, Oahu, Northeast, Wales, Spain and Jamaica.
fn landmark(land: &mut Land, rng: &mut ExeRng, prop: i32, n: i32, a: i32, b: i32, theme: u8) {
    if prop == 5 {
        // Waterfall: a small pool stepped into the hillside.
        let i = idx(a, b);
        land.ty[i] = T_WATER;
        land.set_h(b + a * 51 + 50, 6);
        rng.below(4);
        for k in 0..8 {
            let ka = DX[k] + a;
            let kb = DY[k] + b;
            let ti = ka * N + kb;
            let corner = kb + ka * 51;
            match k {
                0 | 6 => {
                    land.ty[ti as usize] = T_WATER;
                    land.set_h(corner + 50, 6);
                }
                1 | 7 => {
                    land.ty[ti as usize] = T_DEEP_ROUGH;
                    land.set_h(corner + 50, 6);
                }
                2 => {
                    land.ty[ti as usize] = T_WATER;
                    land.set_h(corner + 50, 3);
                }
                _ => {
                    if rng.below(2) != 0 {
                        land.ty[ti as usize] = T_WATER;
                    }
                    land.set_h(corner - 1, 3);
                }
            }
        }
        return;
    }
    if prop == 15 || prop == 12 || prop == 8 {
        let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
        land.objects[o].sub = 0xb;
        if prop == 8 {
            land.objects[o].sub = if rng.below(2) != 0 { 6 } else { 2 };
        } else if prop == 12 {
            land.objects[o].sub = 3;
        }
        land.objects[o].dir = rng.below(4) as u8;
        for k in 0..8 {
            let i = DY[k] + (DX[k] + a) * N + b;
            let t = land.ty_at(i);
            if t == T_BUILDING || t == T_WATER {
                continue;
            }
            if n & 1 == 0 {
                land.ty[i as usize] = T_DEEP_ROUGH;
                land.flags[i as usize] |= ((k as u16 & 1) << 9) | flag::PATH;
            } else {
                land.ty[i as usize] = if prop == 12 { T_ROCKS } else { T_ROUGH };
                if prop == 15 {
                    land.ty[i as usize] = T_WATER;
                } else if k == 0 {
                    let v = b + a * 51;
                    for d in [0, 51, -1, 50] {
                        let hv = land.h(v + d);
                        land.set_h(v + d, hv.wrapping_add(2));
                    }
                }
            }
        }
    }
    if prop == 13 {
        let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
        land.objects[o].sub = 1;
        land.objects[o].dir = rng.below(4) as u8;
        for k in 0..8 {
            let t = land.ty_at(b + DY[k] + (DX[k] + a) * N);
            if t != 0xff && TYPES[t as usize].class == 13 {
                land.objects[o].sub = 10;
            }
            if t == T_WATER {
                land.objects[o].sub = 9;
                break;
            }
        }
        for k in 0..8 {
            let s = land.objects[o].sub;
            let i = DY[k] + (DX[k] + a) * N + b;
            if !(0..NN as i32).contains(&i) {
                continue;
            }
            let iu = i as usize;
            if s == 1 {
                land.flags[iu] |= flag::SPOT;
                land.ty[iu] = T_BRUSH;
            }
            if s == 3 && land.ty[iu] == T_ROUGH {
                land.ty[iu] = T_RAVINE;
            }
            if s == 9 && land.ty[iu] == T_ROUGH && rng.below(3) == 0 {
                land.flags[iu] |= flag::SPOT;
            }
        }
    } else if prop == 6 {
        let o = land.place(rng, a, b, K_LANDMARK, 0, theme);
        land.objects[o].sub = if n & 1 == 0 { 8 } else { 12 };
        land.objects[o].dir = rng.below(4) as u8;
        for k in 0..8 {
            let i = DY[k] + (DX[k] + a) * N + b;
            if !(0..NN as i32).contains(&i) {
                continue;
            }
            if n & 1 == 0 {
                if land.ty[i as usize] != T_WATER {
                    land.flags[i as usize] |= ((k as u16 & 1) << 9) | flag::PATH;
                }
            } else {
                land.flags[i as usize] |= flag::CREEK;
            }
        }
    }
}

impl Land {
    /// Land for a course that was not generated (the demo course, a loaded course file): the terrain's tiles, paths and heights,
    /// no objects. Slot 0 of a Parkland property stands in for the deal.
    pub fn from_terrain(t: &Terrain, theme: u8) -> Land {
        let property = RECORDS.iter().position(|r| r.theme == theme).unwrap_or(0);
        let mut land = Land {
            slot_index: 0,
            slot: Slot { property, acres: 250 },
            difficulty: 1,
            ty: vec![T_ROUGH; NN],
            flags: vec![0; NN],
            layer: vec![TYPES[T_ROUGH as usize].layer; NN],
            var: vec![0; NN],
            height: vec![3; HN],
            original: Vec::new(),
            objects: Vec::new(),
            clubhouse: (-1, -1),
            staff_start: (-1, -1),
            view: (-1, -1),
            margin: 0,
            hill: 48,
            undo: vec![UNDO_NONE; NN],
            refund: vec![0; NN],
            growth: vec![0; NN],
        };
        land.sync_from_terrain(t);
        land.original = land.ty.clone();
        land
    }

    /// Takes the player's terrain edits (tile types, paths, corner heights) into the land, keeping objects and their footprints.
    pub fn sync_from_terrain(&mut self, t: &Terrain) {
        for a in 0..N.min(t.w) {
            for b in 0..N.min(t.h) {
                let i = idx(a, b);
                let o = t.tile_index(a, b);
                let mut ty = t.ty[o];
                // Editor-only and texture ids map back to the game type they stand for.
                ty = match ty {
                    23..=25 => T_WATER,
                    26 => T_GREEN,
                    27..=30 | 34 | 35 => 7,
                    31..=33 => T_ROUGH,
                    v => v,
                };
                let footprint = self.flags[i] & flag::FOOTPRINT != 0;
                if !footprint {
                    self.ty[i] = ty;
                    self.layer[i] = TYPES[ty as usize].layer;
                }
                if t.path_kind.get(o).copied().unwrap_or(0) != 0 {
                    self.flags[i] |= flag::PATH;
                } else if !footprint {
                    self.flags[i] &= !flag::PATH;
                }
            }
        }
        for a in 0..=N.min(t.w) {
            for b in 0..N.min(t.h) {
                // Our corner (cx, cy) is the exe's vertex (cx, cy - 1).
                let v = t.corner[((b + 1) * (t.w + 1) + a) as usize] as i32 + 3;
                if a < 51 {
                    self.height[(a * 51 + b) as usize] = v.clamp(0, 255) as u8;
                }
            }
        }
    }

    /// Writes the land's tiles and corner heights inside a rectangle back to the port's terrain (after placing or removing an
    /// object).
    pub fn write_area(&self, t: &mut Terrain, a0: i32, b0: i32, a1: i32, b1: i32) {
        for a in a0.max(0)..=a1.min(N - 1).min(t.w - 1) {
            for b in b0.max(0)..=b1.min(N - 1).min(t.h - 1) {
                let o = t.tile_index(a, b);
                t.ty[o] = self.ty[idx(a, b)];
                if t.ty[o] == T_WATER {
                    t.variation[o] = 0;
                }
            }
        }
        for cx in a0.max(0)..=(a1 + 1).min(t.w) {
            for cy in b0.max(0)..=(b1 + 1).min(t.h) {
                let v = self.corner(cx, cy - 1) as i32 - 3;
                t.corner[(cy * (t.w + 1) + cx) as usize] = v.clamp(0, crate::terrain::MAX_LEVEL) as i8;
            }
        }
    }

    /// Edge of an object's footprint in tiles as the exe counts it for lookup and removal: an upgraded building (kinds 6 and
    /// up) is one larger, except the Snack Bar, which is placed larger but looked up at its base size (its outer ring is then
    /// neither found by clicks nor cleared on removal, as in the exe).
    pub fn footprint_size(&self, o: &Object) -> i32 {
        let edge = BUILDINGS.get(o.kind as usize).map(|b| b.1).unwrap_or(1);
        if o.kind > 5 && o.kind != K_SNACK_BAR {
            edge + o.sub.clamp(0, 1)
        } else {
            edge
        }
    }

    /// Removes an object as the exe does: its record is freed and its tiles go back to rough (deep rough on the dearer slots;
    /// water under a Marina in Parkland and Tropical), without paths.
    pub fn remove_object(&mut self, i: usize) {
        let Some(o) = self.objects.get(i).copied() else { return };
        if o.kind < 0 {
            return;
        }
        let size = self.footprint_size(&o);
        let theme = self.slot.record().theme;
        self.objects[i].kind = -1;
        let ai = idx(o.a, o.b);
        self.flags[ai] &= 0xfde0;
        for r in 0..size {
            for c in 0..size {
                let t = (o.a + r) * N + o.b + c;
                if !(0..NN as i32).contains(&t) {
                    continue;
                }
                let ty = if o.kind == K_MARINA && (theme == 0 || theme == 2) {
                    T_WATER
                } else if self.slot_index > 11 {
                    T_DEEP_ROUGH
                } else {
                    T_ROUGH
                };
                let tu = t as usize;
                self.flags[tu] &= 0xfbdf;
                self.ty[tu] = ty;
                self.layer[tu] = TYPES[ty as usize].layer;
                self.var[tu] = 0;
            }
        }
    }

    /// Objects joined to the clubhouse: a flood from the clubhouse footprint over path and footprint tiles (the exe's flag 0x40,
    /// which makes a building operational).
    pub fn joined_objects(&self) -> Vec<bool> {
        let mut seen = vec![false; NN];
        let mut stack = Vec::new();
        for o in &self.objects {
            if o.kind == K_CLUBHOUSE {
                let s = self.footprint_size(o);
                for r in 0..s {
                    for c in 0..s {
                        let (a, b) = (o.a + r, o.b + c);
                        if (0..N).contains(&a)
                            && (0..N).contains(&b)
                            && !seen[idx(a, b)]
                            && self.flags[idx(a, b)] & (flag::PATH | flag::FOOTPRINT) != 0
                        {
                            seen[idx(a, b)] = true;
                            stack.push((a, b));
                        }
                    }
                }
            }
        }
        while let Some((a, b)) = stack.pop() {
            // A marked tile passes the flood on only when its type isn't tee, green or fairway (types 0..3).
            if self.ty[idx(a, b)] <= 3 {
                continue;
            }
            for k in [0usize, 2, 4, 6] {
                let (na, nb) = (a + DX[k], b + DY[k]);
                if !(0..N).contains(&na) || !(0..N).contains(&nb) {
                    continue;
                }
                let i = idx(na, nb);
                if !seen[i] && self.flags[i] & (flag::PATH | flag::FOOTPRINT) != 0 {
                    seen[i] = true;
                    stack.push((na, nb));
                }
            }
        }
        self.objects
            .iter()
            .map(|o| {
                let s = self.footprint_size(o);
                o.kind >= 0
                    && (0..s)
                        .any(|r| (0..s).any(|c| (0..N).contains(&(o.a + r)) && (0..N).contains(&(o.b + c)) && seen[idx(o.a + r, o.b + c)]))
            })
            .collect()
    }
}

/// Which building kinds a course has unlocked: kinds 0..5 from the start, then one more each time a hole is completed (holes
/// 2..5 unlock the Putting Green, Snack Bar, Pro Shop and Swim Club; 7..9 the Driving Range, Cart Garage and Marina; 11 and 12
/// the Resort Hotel and Airstrip). Holes 6, 10 and 18 raise the course rank instead. Sandbox games have 17 kinds.
pub fn unlocked_kinds(holes: usize, sandbox: bool) -> i32 {
    if sandbox {
        return 17;
    }
    let mut unlocked = 6;
    for n in 1..=holes as i32 {
        if n != 6 && n != 10 && n > unlocked - 5 && unlocked <= 14 {
            unlocked += 1;
        }
    }
    unlocked
}

/// The port's theme index (0 Parkland, 1 Links, 2 Desert, 3 Tropical) for an exe theme.
pub const fn port_theme(exe_theme: u8) -> usize {
    let t = if exe_theme > 3 { 3 } else { exe_theme };
    [0, 2, 3, 1][t as usize]
}

impl Land {
    /// Converts to the port's Terrain: tile (a, b) becomes (x, y) = (a, b); corner heights become levels above sea level; water
    /// depth goes into the variation byte; paths become gravel paths.
    pub fn to_terrain(&self) -> Terrain {
        let (w, h) = (N, N);
        let mut t = Terrain {
            w,
            h,
            ty: vec![0; NN],
            variation: vec![0; NN],
            set: vec![0; NN],
            corner: vec![0; ((w + 1) * (h + 1)) as usize],
            path_kind: vec![0; NN],
            wall_mask: vec![0; NN],
            desert: self.slot.record().theme == 1,
            path: Vec::new(),
            clubhouse_x: -1,
            clubhouse_y: -1,
            clubhouse_size: 0,
        };
        let mut seed = crate::rng::Rng::new(0x9e37_79b9 ^ self.slot.property as u32);
        for a in 0..N {
            for b in 0..N {
                let i = idx(a, b);
                let o = t.tile_index(a, b);
                t.ty[o] = self.ty[i];
                t.set[o] = seed.range(5) as u8;
                if self.ty[i] == T_WATER {
                    t.variation[o] = self.var[i];
                }
                let building = self.ty[i] == T_BUILDING || self.ty[i] == T_HOME_SITE;
                if self.flags[i] & flag::PATH != 0 && !building {
                    t.path_kind[o] = 1;
                }
            }
        }
        for cy in 0..=h {
            for cx in 0..=w {
                let v = self.corner(cx, cy - 1) as i32 - 3;
                t.corner[(cy * (w + 1) + cx) as usize] = v.clamp(0, crate::terrain::MAX_LEVEL) as i8;
            }
        }
        // The clubhouse sprite stands in the middle of its 4 x 4 footprint.
        t.clubhouse_x = self.clubhouse.0 + 2;
        t.clubhouse_y = self.clubhouse.1 + 2;
        t.clubhouse_size = BUILDINGS[K_CLUBHOUSE as usize].1;
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_matches_the_exe_formula() {
        let mut r = ExeRng { state: 1 };
        // state = 1 * 1103515245 + 12345 = 1103527590; bits 16..30 = 16838; 16838 / 32768 * 100 = 51.38
        assert_eq!(r.below(100), 51);
        assert_eq!(r.state, 1_103_527_590);
    }

    #[test]
    fn offer_is_a_permutation_with_the_exe_rules() {
        for seed in 0..50u32 {
            let mut rng = ExeRng::from_clock(seed * 7919);
            let offer = deal_offer(&mut rng, false);
            let mut seen = [false; 16];
            for s in &offer {
                assert!(!seen[s.property]);
                seen[s.property] = true;
            }
            assert_eq!(RECORDS[offer[0].property].theme, 0);
            let mut themes: Vec<u8> = offer[12..].iter().map(|s| s.record().theme).collect();
            themes.sort();
            assert_eq!(themes, vec![0, 1, 2, 3]);
        }
    }

    #[test]
    fn acres_follow_slot_and_coast() {
        let mut rng = ExeRng::from_clock(1234);
        let offer = deal_offer(&mut rng, false);
        for (i, s) in offer.iter().enumerate() {
            let base = (i as i32 + 4) * 10;
            let bonus = if i > 3 { 20 } else { 10 };
            let want = match s.record().coast {
                0 => base + bonus,
                2 => base - bonus,
                _ => base,
            };
            assert_eq!(s.acres, want);
        }
    }

    #[test]
    fn every_property_generates() {
        for p in 0..16 {
            for d in 0..4 {
                let mut rng = ExeRng::from_clock(1000 + p as u32 * 31 + d as u32);
                let noise = Noise::new(&mut rng);
                let slot = Slot { property: p, acres: 120 };
                let land = generate(7, slot, d, false, &mut rng, &noise);
                let (a, b) = land.clubhouse;
                assert!((15..32).contains(&a) && (15..32).contains(&b));
                assert_eq!(land.tile(a, b), T_BUILDING);
                assert!(land.objects[0].kind == K_CLUBHOUSE);
                assert!(land.height.iter().all(|&h| h <= 17));
                let t = land.to_terrain();
                assert_eq!(t.ty.len(), 2500);
            }
        }
    }

    #[test]
    fn unlocks_follow_holes() {
        assert_eq!(unlocked_kinds(1, false), 6);
        assert_eq!(unlocked_kinds(2, false), 7);
        assert_eq!(unlocked_kinds(5, false), 10);
        assert_eq!(unlocked_kinds(6, false), 10);
        assert_eq!(unlocked_kinds(9, false), 13);
        assert_eq!(unlocked_kinds(12, false), 15);
        assert_eq!(unlocked_kinds(18, false), 15);
    }

    #[test]
    fn owned_square_matches_acres() {
        // 120 acres: margin m is the first count with 1200 >= 4 (25 - m)^2, i.e. 25 - m <= 17, m = 8.
        let mut rng = ExeRng::from_clock(42);
        let noise = Noise::new(&mut rng);
        let land = generate(5, Slot { property: 9, acres: 120 }, 1, false, &mut rng, &noise);
        assert_eq!(land.margin, 8);
        assert_eq!(land.tile(0, 25), T_OUT);
        assert_eq!(land.tile(7, 25), T_OUT);
        assert_ne!(land.tile(8, 25), T_OUT);
        assert_eq!(land.tile(49 - 7, 25), T_OUT);
    }
}

/// What undoing a tile did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Undone {
    /// Nothing could be undone (out of the property or a permanent obstacle).
    Refused,
    /// An item or change was taken back; the amount (units) is given back.
    Refund(i32),
    /// A building stands here: the exe asks before demolishing it, and never refunds it.
    Building(usize),
}

impl Land {
    /// Whether a tile belongs to the property (not out of bounds).
    pub fn in_play(&self, a: i32, b: i32) -> bool {
        (0..N).contains(&a) && (0..N).contains(&b) && self.ty[idx(a, b)] != T_OUT
    }

    /// Price of an item placed on one tile (path, bench, flower bed, willow, scenic bridge), or a reason it cannot go there.
    /// Paths pay half the clearing; willows and bridges cost a flat amount (main frame building tool, 0x41f34e).
    pub fn tile_item_cost(&self, a: i32, b: i32, kind: i32, theme: u8) -> Result<i32, &'static str> {
        if !self.in_play(a, b) {
            return Err("");
        }
        let Some(mut clear) = self.fits(a, b, 1, kind, theme) else { return Err("Can't build there.") };
        if kind == K_PATH {
            clear /= 2;
        }
        let i = idx(a, b);
        let f = self.flags[i];
        Ok(match kind {
            K_PATH if f & flag::PATH != 0 => 0,
            K_BENCH if f & 0x200 != 0 => 0,
            K_BENCH if ![0, 2, 4, 6].iter().any(|&d| self.bench_ok(a, b, d)) => return Err("Can't build bench there."),
            K_FLOWERS if f & 0x1000 != 0 => 0,
            K_WILLOW => 25,
            K_BRIDGE if self.ty[i] != T_WATER => return Err("Can't build bridge there."),
            K_BRIDGE => 100,
            _ => BUILDINGS[kind as usize].2 + clear,
        })
    }

    /// Can a bench at (a, b) face heading d (0x407400): both tiles playable rough-or-worse ground, the neighbour in the property and
    /// free of paths and scenery.
    pub fn bench_ok(&self, a: i32, b: i32, d: i32) -> bool {
        if !(0..N).contains(&a) || !(0..N).contains(&b) {
            return false;
        }
        let ty = self.ty[idx(a, b)];
        if TYPES[ty as usize].desert_group <= 0 || ty == T_BUILDING || ty == T_HOME_SITE {
            return false;
        }
        let (na, nb) = (a + DX[d as usize & 7], b + DY[d as usize & 7]);
        if !self.in_play(na, nb) {
            return false;
        }
        let j = idx(na, nb);
        self.flags[j] & 0x120 == 0 && TYPES[self.ty[j] as usize].desert_group > 0
    }

    /// Puts an item on one tile as the exe's building tool does (paths, benches, flower beds, willows, scenic bridges), recording
    /// the undo. `variant` picks the look (bench 0..4, flowers 0..3, willow 0..6, bridge 0..7). Returns the units charged.
    pub fn place_tile_item(&mut self, a: i32, b: i32, kind: i32, variant: i32, theme: u8) -> Result<i32, &'static str> {
        let cost = self.tile_item_cost(a, b, kind, theme)?;
        let i = idx(a, b);
        match kind {
            K_PATH => {
                if self.flags[i] & flag::PATH == 0 {
                    self.flags[i] |= flag::PATH;
                    if self.ty[i] == T_WATER {
                        self.flags[i] &= !flag::SPOT;
                    }
                    for d in [0usize, 2, 4, 6] {
                        let (na, nb) = (a + DX[d], b + DY[d]);
                        if !(0..N).contains(&na) || !(0..N).contains(&nb) {
                            continue;
                        }
                        let j = idx(na, nb);
                        if (self.ty[j] == T_BUILDING || self.ty[j] == T_HOME_SITE)
                            && self.objects.get(self.var[j] as usize).map(|o| o.kind >= 6).unwrap_or(false)
                        {
                            self.flags[j] |= flag::PATH;
                        }
                    }
                    self.undo[i] = 0x80;
                    self.refund[i] = cost as i8;
                }
            }
            K_BENCH => {
                if self.flags[i] & 0x200 == 0 {
                    self.flags[i] = (self.flags[i] & !0x1000) | 0x220;
                    self.var[i] = (variant % 5) as u8;
                    self.undo[i] = 0x81;
                    self.refund[i] = cost as i8;
                }
            }
            K_FLOWERS => {
                if self.flags[i] & 0x1000 != 0 {
                    self.var[i] = variant as u8;
                } else {
                    self.flags[i] |= 0x5000;
                    self.growth[i] = 0;
                    self.flags[i] &= !(0x0100 | 0x0200 | 0x0800);
                    self.var[i] = variant as u8;
                    self.ty[i] = T_ROUGH;
                    self.layer[i] = TYPES[T_ROUGH as usize].layer;
                    self.undo[i] = 0x82;
                    self.refund[i] = cost as i8;
                }
            }
            K_WILLOW => {
                self.ty[i] = T_ELM;
                self.layer[i] = TYPES[T_ELM as usize].layer;
                self.growth[i] = 0;
                self.var[i] = (variant + 1) as u8;
                self.flags[i] = (self.flags[i] & !0x1800) | 0x4100;
                self.undo[i] = 0x90;
                self.refund[i] = 25;
            }
            K_BRIDGE => {
                self.flags[i] = (self.flags[i] & 0xe7f8) | (variant & 7) as u16 | 0x120;
                self.undo[i] = 0x93;
                self.refund[i] = 100;
            }
            _ => return Err(""),
        }
        Ok(cost)
    }

    /// Takes back what was done on a tile (the exe's right-click undo, 0x40a4e0).
    /// The object whose footprint covers a tile.
    pub fn object_on(&self, a: i32, b: i32) -> Option<usize> {
        if !(0..N).contains(&a) || !(0..N).contains(&b) || self.flags[idx(a, b)] & flag::FOOTPRINT == 0 {
            return None;
        }
        self.objects.iter().position(|o| {
            let s = self.footprint_size(o);
            o.kind >= 0 && (o.a..o.a + s).contains(&a) && (o.b..o.b + s).contains(&b)
        })
    }

    /// Records a terrain change for undo (0x41fee2): the tile's old type and the amount to give back. The exe keeps the
    /// amount in a signed byte, so very dear changes wrap (marsh to water, 150, gives back -106).
    pub fn record_paint(&mut self, a: i32, b: i32, old: u8, cost: i32) {
        if (0..N).contains(&a) && (0..N).contains(&b) {
            let i = idx(a, b);
            self.undo[i] = old;
            self.refund[i] = cost as i8;
        }
    }

    pub fn undo_tile(&mut self, a: i32, b: i32) -> Undone {
        if !self.in_play(a, b) {
            return Undone::Refused;
        }
        let i = idx(a, b);
        self.flags[i] &= !0x100;
        if self.flags[i] & 0x8000 != 0 {
            return Undone::Refused;
        }
        let on_object = self.flags[i] & flag::FOOTPRINT != 0;
        if on_object {
            if let Some(o) = self.objects.iter().position(|o| {
                let s = BUILDINGS.get(o.kind as usize).map(|b| b.1).unwrap_or(1);
                o.kind >= 0 && (o.a..o.a + s).contains(&a) && (o.b..o.b + s).contains(&b)
            }) {
                return Undone::Building(o);
            }
        }
        let code = self.undo[i];
        if code == UNDO_NONE {
            let f = self.flags[i];
            if f & 0x200 != 0 {
                self.flags[i] &= !0x200;
            } else if f & flag::PATH != 0 {
                self.flags[i] &= !flag::PATH;
            } else if f & 0x1000 != 0 {
                self.flags[i] &= !0x1000;
            }
            return Undone::Refund(0);
        }
        if code < 0x7d {
            self.ty[i] = code;
            self.layer[i] = TYPES[(code as usize).min(22)].layer;
        } else {
            match code & 0x7f {
                0x10 => {
                    self.ty[i] = T_ROUGH;
                    self.layer[i] = TYPES[T_ROUGH as usize].layer;
                }
                0x13 | 0 => self.flags[i] &= !flag::PATH,
                1 => self.flags[i] &= !0x200,
                _ => {
                    if self.flags[i] & 0x1000 != 0 {
                        self.flags[i] &= !0x1000;
                    } else {
                        self.flags[i] &= !0x1220;
                    }
                }
            }
        }
        let r = self.refund[i] as i32;
        self.undo[i] = UNDO_NONE;
        self.refund[i] = 0;
        Undone::Refund(r)
    }
}

#[cfg(test)]
mod tile_item_tests {
    use super::*;
    use crate::terrain::Terrain;

    fn rough_land() -> Land {
        let t = Terrain {
            w: N,
            h: N,
            ty: vec![T_ROUGH; NN],
            variation: vec![0; NN],
            set: vec![0; NN],
            corner: vec![0; ((N + 1) * (N + 1)) as usize],
            path_kind: vec![0; NN],
            wall_mask: vec![0; NN],
            desert: false,
            path: Vec::new(),
            clubhouse_x: -1,
            clubhouse_y: -1,
            clubhouse_size: 0,
        };
        Land::from_terrain(&t, 0)
    }

    #[test]
    fn benches_flowers_and_undo() {
        let mut l = rough_land();
        assert_eq!(l.place_tile_item(10, 10, K_BENCH, 3, 0), Ok(2));
        assert_eq!(l.flags[idx(10, 10)] & 0x220, 0x220);
        // A second bench on the same tile is free and changes nothing.
        assert_eq!(l.place_tile_item(10, 10, K_BENCH, 1, 0), Ok(0));
        assert_eq!(l.undo_tile(10, 10), Undone::Refund(2));
        assert_eq!(l.flags[idx(10, 10)] & 0x200, 0);
        // Benches need rough-or-worse ground with a free neighbour: none on fairway.
        l.ty[idx(20, 20)] = 2;
        assert!(l.place_tile_item(20, 20, K_BENCH, 0, 0).is_err());
        // A flower bed turns the tile to rough and grows; willows cost a flat 25 and become elms.
        assert_eq!(l.place_tile_item(12, 12, K_FLOWERS, 0, 0), Ok(5));
        assert_eq!(l.flags[idx(12, 12)] & 0x5000, 0x5000);
        assert_eq!(l.place_tile_item(14, 14, K_WILLOW, 2, 0), Ok(25));
        assert_eq!(l.ty[idx(14, 14)], T_ELM);
        assert_eq!(l.undo_tile(14, 14), Undone::Refund(25));
        assert_eq!(l.ty[idx(14, 14)], T_ROUGH);
        // Bridges only on water.
        assert!(l.place_tile_item(16, 16, K_BRIDGE, 0, 0).is_err());
    }

    #[test]
    fn paths_join_buildings_but_stop_on_fairway() {
        let mut l = rough_land();
        l.objects.clear();
        let mut put = |l: &mut Land, kind: i32, a: i32, b: i32| {
            let o = Object { kind, a, b, dir: 0, flags: 0, sub: 0, val: 0 };
            let s = l.footprint_size(&o);
            for r in 0..s {
                for c in 0..s {
                    l.flags[idx(a + r, b + c)] |= flag::FOOTPRINT;
                }
            }
            l.objects.push(o);
        };
        put(&mut l, K_CLUBHOUSE, 10, 10);
        put(&mut l, K_SNACK_BAR, 20, 10);
        for a in 14..20 {
            l.flags[idx(a, 10)] |= flag::PATH;
        }
        assert_eq!(l.joined_objects(), vec![true, true]);
        // A fairway tile on the path still gets marked but passes nothing on.
        l.ty[idx(16, 10)] = 2;
        assert_eq!(l.joined_objects(), vec![true, false]);
    }
}
