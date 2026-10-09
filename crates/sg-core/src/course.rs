//! The course as the golfers see it, in the exe's own layout: 50 x 50 tiles indexed a*50 + b (a = x tile, b = y tile), the
//! per-type terrain table, tile flags, corner heights (51 x 51, sea level 3), the walls between tiles of different height,
//! and the objects. Built from `land::Land` whenever the course changes (the exe's refresh routine 0x42f7a0).

use crate::geom::{len, DX, DY, UNIT};
use crate::land::{Land, Object, BUILDINGS};

pub const N: i32 = 50;
pub const NN: usize = 2500;

/// Tile types the golfer code tests by number.
pub mod t {
    pub const TEE: u8 = 0;
    pub const GREEN: u8 = 1;
    pub const FAIRWAY: u8 = 2;
    pub const ROUGH: u8 = 4;
    pub const SAND: u8 = 7;
    pub const POT: u8 = 9;
    pub const RAVINE: u8 = 10;
    pub const ROCKS: u8 = 12;
    pub const WATER: u8 = 17;
    pub const MARSH: u8 = 19;
    pub const OUT: u8 = 20;
    pub const HOME: u8 = 21;
    pub const BUILDING: u8 = 22;
}

/// Tile flag bits (u16 per tile).
pub mod f {
    /// Low 5 bits: hole number on a cup green or tee; object kind + 1 on an object's anchor tile.
    pub const LOW: u16 = 0x1f;
    pub const PATH: u16 = 0x20;
    pub const JOINED: u16 = 0x40;
    pub const CUP: u16 = 0x80;
    pub const SCENIC: u16 = 0x100;
    pub const BENCH: u16 = 0x200;
    pub const FOOTPRINT: u16 = 0x400;
    pub const WEED: u16 = 0x800;
    pub const FLOWERS: u16 = 0x1000;
    pub const GROWING: u16 = 0x4000;
    pub const OBSTACLE: u16 = 0x8000;
}

/// One row of the runtime terrain table (0x578350, 0x30 bytes per type).
#[derive(Clone, Copy, Debug)]
pub struct TypeRow {
    /// +0x20 bounce
    pub bounce: i32,
    /// +0x21 roll
    pub roll: i32,
    /// +0x22 hazard level (negative on greens)
    pub hazard: i8,
    /// +0x25 walking cost
    pub walk: i32,
    /// +0x26 class
    pub class: u8,
    /// +0x27 scenery group
    pub group: u8,
    /// +0x2c flags: 1 level table, 2 flat at the lowest corner, 4 flat at the highest, 8 height 0
    pub flags: u32,
}

const fn row(bounce: i32, roll: i32, hazard: i8, walk: i32, class: u8, group: u8, flags: u32) -> TypeRow {
    TypeRow { bounce, roll, hazard, walk, class, group, flags }
}

pub const TYPES: [TypeRow; 23] = [
    row(4, 3, 0, 2, 0, 0, 4),      // tee
    row(3, 3, -1, 1, 1, 1, 0),     // green
    row(4, 3, 0, 1, 2, 2, 0),      // fairway
    row(5, 4, 0, 1, 2, 15, 0),     // firm fairway
    row(2, 2, 1, 2, 4, 3, 0),      // rough
    row(1, 2, 2, 3, 4, 17, 0),     // deep rough
    row(2, 2, 2, 3, 4, 4, 0),      // mounds
    row(1, 1, 3, 4, 7, 5, 0),      // sand trap
    row(2, 2, 2, 3, 8, 5, 0),      // waste bunker
    row(1, 1, 5, 5, 7, 5, 2),      // pot bunker
    row(2, 2, 5, 3, 4, 6, 0),      // ravine
    row(1, 1, 3, 3, 4, 6, 0),      // brush
    row(5, 1, 3, 4, 4, 9, 0x100),  // rocks
    row(2, 2, 4, 4, 13, 7, 0x400), // tree
    row(2, 2, 4, 4, 13, 7, 0x200), // pine
    row(2, 2, 4, 4, 13, 6, 0x200), // palm
    row(2, 2, 4, 4, 13, 7, 0x400), // elm
    row(0, 0, 8, 16, 17, 8, 2),    // water
    row(2, 1, 3, 4, 4, 14, 8),     // wetlands
    row(2, 1, 3, 4, 4, 14, 8),     // marsh
    row(2, 2, 8, 12, 18, 6, 0),    // out of bounds
    row(2, 2, 4, 4, 18, 16, 5),    // building (home site)
    row(2, 2, 4, 4, 18, 16, 5),    // building
];

pub fn idx(a: i32, b: i32) -> usize {
    (a * N + b) as usize
}

pub fn inside(a: i32, b: i32) -> bool {
    (0..N).contains(&a) && (0..N).contains(&b)
}

#[derive(Clone, Debug)]
pub struct Course {
    pub ty: Vec<u8>,
    pub flags: Vec<u16>,
    /// Per-tile object byte: object index on footprints, else a variant byte.
    pub obj: Vec<u8>,
    /// Walking cost per tile (from the type's +0x25 when the tile was set).
    pub walk: Vec<i32>,
    /// Corner heights 51 x 51, index a*51 + b.
    pub height: Vec<u8>,
    /// Flattened level per tile for buildings (0x543018).
    pub level_table: Vec<i32>,
    /// Wall bits per tile: 1 << d for d in 0, 2, 4, 6 when the neighbour that way stands higher (0x5619a0).
    pub walls: Vec<u8>,
    pub objects: Vec<Object>,
    /// Runtime hazard bytes per type; the shot planner changes a few of them while it runs.
    pub hazard: [i8; 23],
    /// Exe theme 0 Parkland, 1 Desert, 2 Tropical, 3 Links.
    pub theme: u8,
    /// Clubhouse anchor tile (0x578150/0x578154).
    pub door: (i32, i32),
    /// Offer slot of the property (removal turns tiles to deep rough above 11).
    pub slot_index: usize,
    /// Highest upgrade level + 1 per object kind (0x5a8c38), and the same over operational objects (0x543ca0).
    pub level: [i32; 20],
    pub oper: [i32; 20],
    /// Landing counter per tile (0x53ea24).
    pub landing: Vec<i8>,
    /// Weed growth per tile (0x578804).
    pub growth: Vec<u8>,
    /// Happy and unhappy golfer counters per tile (0x5a6378, 0x56c7e4).
    pub happy: Vec<u8>,
    pub unhappy: Vec<u8>,
    /// Game flag 1: editor view, slopes read as flat.
    pub editor: bool,
}

impl Default for Course {
    fn default() -> Self {
        Course {
            ty: vec![t::ROUGH; NN],
            flags: vec![0; NN],
            obj: vec![0; NN],
            walk: vec![2; NN],
            height: vec![3; 51 * 51],
            level_table: vec![0; NN],
            walls: vec![0; NN],
            objects: Vec::new(),
            hazard: std::array::from_fn(|i| TYPES[i].hazard),
            theme: 0,
            door: (0, 0),
            slot_index: 0,
            level: [0; 20],
            oper: [0; 20],
            landing: vec![0; NN],
            growth: vec![0; NN],
            happy: vec![0; NN],
            unhappy: vec![0; NN],
            editor: false,
        }
    }
}

impl Course {
    /// Takes tiles, flags, heights and objects from the land, keeping the per-tile counters, then rebuilds levels and walls.
    pub fn sync(&mut self, land: &Land) {
        self.ty.copy_from_slice(&land.ty);
        // Weed bits belong to the staff routines; keep the ones already here.
        for i in 0..NN {
            let keep = self.flags[i] & (f::WEED | f::GROWING);
            self.flags[i] = (land.flags[i] & !(f::WEED | f::GROWING)) | keep;
        }
        self.obj.copy_from_slice(&land.var);
        for i in 0..NN {
            self.walk[i] = land.layer[i] as i32;
        }
        self.height.copy_from_slice(&land.height);
        self.objects = land.objects.clone();
        self.theme = land.slot.record().theme;
        if land.clubhouse.0 >= 0 {
            self.door = land.clubhouse;
        }
        self.slot_index = land.slot_index;
        let joined = land.joined_objects();
        self.level = [0; 20];
        self.oper = [0; 20];
        for (i, o) in self.objects.iter_mut().enumerate() {
            if o.kind < 0 {
                continue;
            }
            // Operational: joined to the clubhouse by path (kinds 0..5 and 17.. always count).
            if o.kind <= 5 || o.kind >= 17 || joined.get(i).copied().unwrap_or(false) {
                o.flags |= 0x40;
            } else {
                o.flags &= !0x40;
            }
        }
        for o in &self.objects {
            if o.kind > 5 && (o.kind as usize) < 20 {
                let k = o.kind as usize;
                self.level[k] = self.level[k].max((o.sub + 1).min(99));
                if o.flags & 0x40 != 0 {
                    self.oper[k] = self.oper[k].max((o.sub + 1).min(99));
                }
            }
        }
        if self.level[7] != 0 {
            // The exe's quirk: an existing snack bar zeroes its operational level entry.
            self.oper[7] = 0;
        }
        self.refresh();
    }

    pub fn ty_at(&self, a: i32, b: i32) -> u8 {
        if inside(a, b) {
            self.ty[idx(a, b)]
        } else {
            t::OUT
        }
    }

    pub fn row(&self, ty: u8) -> &TypeRow {
        &TYPES[(ty as usize).min(22)]
    }

    /// Hazard byte of a type, as the table holds it right now.
    pub fn h(&self, ty: u8) -> i32 {
        self.hazard[(ty as usize).min(22)] as i32
    }

    pub fn flags_at(&self, a: i32, b: i32) -> u16 {
        if inside(a, b) {
            self.flags[idx(a, b)]
        } else {
            0
        }
    }

    /// Off the map or out of bounds (0x40bf60).
    pub fn oob(&self, a: i32, b: i32) -> bool {
        !inside(a, b) || self.ty[idx(a, b)] == t::OUT
    }

    /// Type under a map point, out of bounds (20) off the map (0x40bfa0).
    pub fn tile_type(&self, x: i32, y: i32) -> u8 {
        let (a, b) = (x >> 10, y >> 10);
        if self.oob(a, b) {
            t::OUT
        } else {
            self.ty[idx(a, b)]
        }
    }

    /// Stored corner height, sea level at the edge of the map and deep in the out-of-bounds land (0x40c170).
    pub fn raw_corner(&self, a: i32, b: i32) -> i32 {
        if !inside(a, b) {
            return 3;
        }
        if self.ty[idx(a, b)] == t::OUT && self.oob(a - 1, b) && self.oob(a, b + 1) {
            return 3;
        }
        self.height[(a * 51 + b) as usize] as i32
    }

    /// Highest and lowest of a tile's four raw corners (0x42f4b0).
    pub fn corner_range(&self, a: i32, b: i32) -> (i32, i32) {
        let c = [self.raw_corner(a, b), self.raw_corner(a + 1, b), self.raw_corner(a + 1, b - 1), self.raw_corner(a, b - 1)];
        (*c.iter().max().unwrap(), *c.iter().min().unwrap())
    }

    /// Corner k (1 = +x -y, 3 = +x +y, 5 = -x +y, 7 = -x -y) of tile (a, b) as the ground uses it: flat tiles take their
    /// lowest or highest corner, buildings their level, wetlands sea level (0x40bfe0).
    pub fn corner(&self, a: i32, b: i32, k: i32) -> i32 {
        if !inside(a, b) || k & 1 == 0 {
            return 3;
        }
        let i = idx(a, b);
        let fl = self.row(self.ty[i]).flags;
        if fl & 2 != 0 {
            return if fl & 1 != 0 { self.level_table[i] } else { self.corner_range(a, b).1 };
        }
        if fl & 4 != 0 {
            return if fl & 1 != 0 { self.level_table[i] } else { self.corner_range(a, b).0 };
        }
        if fl & 8 != 0 {
            return 3;
        }
        match k & 7 {
            1 => self.raw_corner(a + 1, b - 1),
            3 => self.raw_corner(a + 1, b),
            5 => self.raw_corner(a, b),
            _ => self.raw_corner(a, b - 1),
        }
    }

    /// Rebuilds the building levels and the walls (0x42f7a0).
    pub fn refresh(&mut self) {
        for v in self.level_table.iter_mut() {
            *v = 0;
        }
        for a in 0..N {
            for b in 0..N {
                let i = idx(a, b);
                let fl = self.row(self.ty[i]).flags;
                if fl & 6 != 0 {
                    let (hi, lo) = self.corner_range(a, b);
                    if fl & 2 != 0 {
                        self.level_table[i] = lo;
                    }
                    if fl & 4 != 0 {
                        self.level_table[i] = hi;
                    }
                }
            }
        }
        loop {
            let mut changed = false;
            for a in 0..N {
                for b in 0..N {
                    let i = idx(a, b);
                    let r = *self.row(self.ty[i]);
                    if r.flags & 1 == 0 {
                        continue;
                    }
                    for d in [0usize, 2, 4, 6] {
                        let (na, nb) = (a + DX[d], b + DY[d]);
                        if self.oob(na, nb) {
                            continue;
                        }
                        let j = idx(na, nb);
                        if self.ty[j] != self.ty[i] {
                            continue;
                        }
                        if r.flags & 2 != 0 && self.level_table[j] < self.level_table[i] {
                            self.level_table[i] = self.level_table[j];
                            changed = true;
                        }
                        if r.flags & 4 != 0 && (r.group != 16 || self.obj[j] == self.obj[i]) && self.level_table[j] > self.level_table[i] {
                            self.level_table[i] = self.level_table[j];
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for a in 0..N {
            for b in 0..N {
                let mut w = 0u8;
                for d in [0i32, 2, 4, 6] {
                    let (na, nb) = (a + DX[d as usize], b + DY[d as usize]);
                    if self.oob(na, nb) {
                        continue;
                    }
                    let k = d - 1;
                    if self.corner(a, b, k & 7) < self.corner(na, nb, (k + 6) & 7) {
                        w |= 1 << d;
                    }
                    if self.corner(a, b, (k + 2) & 7) < self.corner(na, nb, (k + 4) & 7) {
                        w |= 1 << d;
                    }
                }
                self.walls[idx(a, b)] = w;
            }
        }
        // The exe finishes by touching the first tile's bits.
        self.walls[0] = (self.walls[0] & 0xfd) | 8;
    }

    /// Ground height under a map point, 16 per height step above sea level (0x42fa30).
    pub fn ground(&self, x: i32, y: i32) -> i32 {
        let (a, b) = (x >> 10, y >> 10);
        if !inside(a, b) {
            return 0;
        }
        let fl = self.row(self.ty[idx(a, b)]).flags;
        if fl & 8 != 0 {
            return 0;
        }
        if fl & 2 != 0 {
            return (self.corner_range(a, b).1 - 3) * 16;
        }
        if fl & 4 != 0 {
            return (self.corner_range(a, b).0 - 3) * 16;
        }
        let c5 = self.corner(a, b, 5) - 3;
        let c7 = self.corner(a, b, 7) - 3;
        let c1 = self.corner(a, b, 1) - 3;
        let c3 = self.corner(a, b, 3) - 3;
        if c5 == c7 && c5 == c1 && c5 == c3 {
            return c5 * 16;
        }
        let fx = x - a * UNIT;
        let fy = y - b * UNIT;
        let v = (((UNIT - fx) * c7 + fx * c1) * (UNIT - fy) + ((UNIT - fx) * c5 + fx * c3) * fy).wrapping_mul(16);
        v / 1024 / 1024
    }

    fn sloped(&self, a: i32, b: i32) -> bool {
        if self.editor || !inside(a, b) {
            return false;
        }
        let ty = self.ty[idx(a, b)];
        ty != t::SAND && ty != t::POT && self.row(ty).flags & 0xe == 0
    }

    /// Rise toward +x at a point (0x40c2f0).
    pub fn slope_x(&self, x: i32, y: i32) -> i32 {
        let (a, b) = (x >> 10, y >> 10);
        if !self.sloped(a, b) {
            return 0;
        }
        if y > b * UNIT + 512 {
            self.corner(a, b, 3) - self.corner(a, b, 5)
        } else {
            self.corner(a, b, 1) - self.corner(a, b, 7)
        }
    }

    /// Rise toward +y at a point (0x40c3a0).
    pub fn slope_y(&self, x: i32, y: i32) -> i32 {
        let (a, b) = (x >> 10, y >> 10);
        if !self.sloped(a, b) {
            return 0;
        }
        if x > a * UNIT + 512 {
            self.corner(a, b, 3) - self.corner(a, b, 1)
        } else {
            self.corner(a, b, 5) - self.corner(a, b, 7)
        }
    }

    /// Rise in heading d at a point, positive uphill (0x40c450).
    pub fn rise(&self, x: i32, y: i32, d: i32) -> i32 {
        let d = (d & 7) as usize;
        let (mut sx, mut sy) = (self.slope_x(x, y), self.slope_y(x, y));
        if d & 1 == 1 {
            sx = sx.clamp(-1, 1);
            sy = sy.clamp(-1, 1);
        }
        DY[d] * sy + DX[d] * sx
    }

    /// Footprint edge of an object as lookups count it (0x40df80): upgraded buildings grow, except the snack bar.
    pub fn object_edge(&self, kind: i32) -> i32 {
        let e = BUILDINGS.get(kind as usize).map(|b| b.1).unwrap_or(1);
        if kind > 5 && kind != 7 {
            e - 1 + self.level.get(kind as usize).copied().unwrap_or(0)
        } else {
            e
        }
    }

    /// Object whose square covers tile (a, b), or -1 (0x40df80).
    pub fn object_at(&self, a: i32, b: i32) -> i32 {
        for (i, o) in self.objects.iter().enumerate() {
            if o.kind == -1 {
                continue;
            }
            let e = self.object_edge(o.kind);
            if o.a <= a && a < o.a + e && o.b <= b && b < o.b + e {
                return i as i32;
            }
        }
        -1
    }

    /// Nearest object of a kind to a map point and its distance (0x40ddb0). For kinds above 5 an object that is not operational
    /// ends the search (the exe's early return).
    pub fn nearest(&self, kind: i32, x: i32, y: i32) -> (i32, i32) {
        let mut best = -1;
        let mut bd = 0xffff;
        let half = BUILDINGS.get(kind as usize).map(|b| b.1).unwrap_or(1) / 2;
        for (i, o) in self.objects.iter().enumerate() {
            if o.kind != kind {
                continue;
            }
            if kind > 5 && o.flags & 0x40 == 0 {
                return (best, bd);
            }
            let d = len((o.a + half) * UNIT - x + 512, (o.b + half) * UNIT - y + 512);
            if d < bd {
                best = i as i32;
                bd = d;
            }
        }
        (best, bd)
    }

    /// Nearest bench tile within 4 tiles closer than r tiles (0x40de70).
    pub fn nearest_bench(&self, x: i32, y: i32, r: i32) -> Option<(i32, i32)> {
        let (ca, cb) = (x >> 10, y >> 10);
        let mut bd = r << 10;
        let mut out = None;
        for a in ca - 4..=ca + 4 {
            for b in cb - 4..=cb + 4 {
                if self.oob(a, b) || self.flags[idx(a, b)] & f::BENCH == 0 {
                    continue;
                }
                let d = len(a * UNIT - x + 512, b * UNIT - y + 512);
                if d < bd {
                    bd = d;
                    out = Some((a, b));
                }
            }
        }
        out
    }

    /// Can a golfer sit on the bench at (a, b) facing heading d (0x407400).
    pub fn bench_ok(&self, a: i32, b: i32, d: i32) -> bool {
        if !inside(a, b) {
            return false;
        }
        let ty = self.ty[idx(a, b)];
        if self.h(ty) <= 0 || ty == t::BUILDING || ty == t::HOME {
            return false;
        }
        let (na, nb) = (a + DX[d as usize & 7], b + DY[d as usize & 7]);
        if self.oob(na, nb) {
            return false;
        }
        let j = idx(na, nb);
        self.flags[j] & 0x120 == 0 && self.h(self.ty[j]) > 0
    }

    /// Landmarks near a point: bit (kind & 3) for every landmark of a kind below 16 within its reach (0x407000).
    pub fn landmark_bits(&self, x: i32, y: i32) -> u32 {
        let mut bits = 0;
        for o in &self.objects {
            if o.kind == 4 && o.sub < 16 && crate::geom::tdist(x, y, o.a, o.b) < ((o.sub * 5 + 40) * 5) / 3 {
                bits |= 1 << (o.sub & 3);
            }
        }
        bits
    }
}
